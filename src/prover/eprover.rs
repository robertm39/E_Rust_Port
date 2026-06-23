use std::fmt;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::basics::error::{check_option_letter_string, Diagnostic, ErrorCode};
use crate::basics::os_wrapper::{get_system_phys_memory, set_memory_limit};
use crate::basics::verbose::set_verbose_level;
use crate::inout::commandline::{
    get_bool_arg, get_int_arg, get_int_arg_check_range, print_options, CommandLineState, ParsedOpt,
};
use crate::inout::output::set_output_level;
use crate::inout::scanner::IoFormat;
use crate::inout::signals::{set_hard_time_limit, set_schedule_time_limit, set_soft_time_limit};
use crate::prover::options::{EProverOption, EPROVER_OPTIONS};
use crate::prover::version::{self, E_NICKNAME, PROGRAM_NAME, VERSION};

const MEGA: u64 = 1_048_576;
const DEFAULT_DELETE_BAD_LIMIT: i64 = i64::MAX;
const DEFAULT_EQDEF_INCRLIMIT: i64 = 20;
const DEFAULT_EQDEF_MAXCLAUSES: i64 = 20_000;
const DEFAULT_OUTPUT_DESCRIPTOR: &str = "eigEIG";
const DEFAULT_FILTER_DESCRIPTOR: &str = "Fc";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EProverAction {
    Help,
    Version,
    Run(Box<EProverConfig>),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum DocOutputFormat {
    #[default]
    NoFormat = 0,
    Lop = 1,
    Pcl = 2,
    Tstp = 3,
    Tptp = 4,
    Xml = 5,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EquationPrintConfig {
    pub use_infix: bool,
    pub full_equational_rep: bool,
    pub print_oriented: bool,
}

impl Default for EquationPrintConfig {
    fn default() -> Self {
        Self {
            use_infix: true,
            full_equational_rep: false,
            print_oriented: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PclOutputConfig {
    pub full_terms: bool,
    pub compact: bool,
    pub shell_level: i64,
}

impl Default for PclOutputConfig {
    fn default() -> Self {
        Self {
            full_terms: true,
            compact: false,
            shell_level: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum AcHandling {
    None = 0,
    #[default]
    DiscardAll = 1,
    KeepUnits = 2,
    KeepOrientable = 3,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GoalDefinitionConfig {
    pub positive: bool,
    pub negative: bool,
    pub subterms: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreprocessingConfig {
    pub no_preprocessing: bool,
    pub eqdef_maxclauses: i64,
    pub eqdef_incrlimit: i64,
    pub goal_definitions: GoalDefinitionConfig,
    pub relevance_prune_level: i64,
    pub presat_interreduction: bool,
    pub ac_handling: AcHandling,
    pub ac_res_aggressive: bool,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            no_preprocessing: false,
            eqdef_maxclauses: DEFAULT_EQDEF_MAXCLAUSES,
            eqdef_incrlimit: DEFAULT_EQDEF_INCRLIMIT,
            goal_definitions: GoalDefinitionConfig::default(),
            relevance_prune_level: 0,
            presat_interreduction: false,
            ac_handling: AcHandling::DiscardAll,
            ac_res_aggressive: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EProverConfig {
    pub files: Vec<String>,
    pub output_file: Option<String>,
    pub output_level: i64,
    pub verbose: i64,
    pub proof_object_level: i64,
    pub proof_output: i64,
    pub force_derivation_output: i64,
    pub training_examples: Option<i64>,
    pub parse_format: IoFormat,
    pub output_format: IoFormat,
    pub doc_output_format: DocOutputFormat,
    pub equation_print: EquationPrintConfig,
    pub pcl_output: PclOutputConfig,
    pub saturated_output_descriptor: String,
    pub filter_saturated_descriptor: String,
    pub select_strategy: Option<String>,
    pub print_strategy: Option<String>,
    pub parse_strategy_file: Option<String>,
    pub sine: Option<String>,
    pub preprocessing: PreprocessingConfig,
    pub strategy_scheduling: bool,
    pub schedule_cores: i64,
    pub serialize_schedule: bool,
    pub force_preprocessing_schedule: bool,
    pub step_limit: i64,
    pub answer_limit: i64,
    pub processed_set_limit: i64,
    pub unprocessed_limit: i64,
    pub total_clause_set_limit: i64,
    pub generated_limit: i64,
    pub term_bank_insert_limit: i64,
    pub cpu_limit: Option<i64>,
    pub soft_cpu_limit: Option<i64>,
    pub schedule_time_limit: Option<i64>,
    pub memory_limit: u64,
    pub delete_bad_limit: i64,
    pub flags: EProverFlags,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EProverFlags {
    bits: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EProverFlag {
    SyntaxOnly = 1 << 0,
    PrintPid = 1 << 1,
    PrintVersion = 1 << 2,
    Auto = 1 << 3,
    DeterministicRewriteSort = 1 << 4,
    DeterministicNewSort = 1 << 5,
    PrintFormulas = 1 << 6,
    PruneOnly = 1 << 7,
    CnfOnly = 1 << 8,
    RequireNonempty = 1 << 9,
    ProofStatistics = 1 << 10,
    FullDerivation = 1 << 11,
    RecordGivenClauses = 1 << 12,
    PrintStatistics = 1 << 13,
    PrintDetailedStatistics = 1 << 14,
    PrintSaturated = 1 << 15,
    PrintSaturatedInfo = 1 << 16,
    FilterSaturated = 1 << 17,
    ResourceInfo = 1 << 18,
    ConjecturesAreQuestions = 1 << 19,
}

impl EProverFlags {
    pub fn set(&mut self, flag: EProverFlag) {
        self.bits |= flag as u32;
    }

    #[must_use]
    pub const fn contains(self, flag: EProverFlag) -> bool {
        (self.bits & flag as u32) != 0
    }
}

impl Default for EProverConfig {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            output_file: None,
            output_level: 1,
            verbose: 0,
            proof_object_level: 0,
            proof_output: 0,
            force_derivation_output: 0,
            training_examples: None,
            parse_format: IoFormat::Auto,
            output_format: IoFormat::Lop,
            doc_output_format: DocOutputFormat::NoFormat,
            equation_print: EquationPrintConfig::default(),
            pcl_output: PclOutputConfig::default(),
            saturated_output_descriptor: DEFAULT_OUTPUT_DESCRIPTOR.to_owned(),
            filter_saturated_descriptor: DEFAULT_FILTER_DESCRIPTOR.to_owned(),
            select_strategy: None,
            print_strategy: None,
            parse_strategy_file: None,
            sine: None,
            preprocessing: PreprocessingConfig::default(),
            strategy_scheduling: false,
            schedule_cores: 1,
            serialize_schedule: false,
            force_preprocessing_schedule: true,
            step_limit: i64::MAX,
            answer_limit: 1,
            processed_set_limit: i64::MAX,
            unprocessed_limit: i64::MAX,
            total_clause_set_limit: i64::MAX,
            generated_limit: i64::MAX,
            term_bank_insert_limit: i64::MAX,
            cpu_limit: None,
            soft_cpu_limit: None,
            schedule_time_limit: None,
            memory_limit: 0,
            delete_bad_limit: DEFAULT_DELETE_BAD_LIMIT,
            flags: EProverFlags::default(),
        }
    }
}

#[derive(Debug)]
pub enum EProverError {
    Diagnostic(Diagnostic),
    Io(io::Error),
}

impl EProverError {
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.code(),
            Self::Io(_) => ErrorCode::OTHER_ERROR,
        }
    }

    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Diagnostic(diagnostic) => diagnostic.message().to_owned(),
            Self::Io(error) => error.to_string(),
        }
    }
}

impl fmt::Display for EProverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for EProverError {}

impl From<Diagnostic> for EProverError {
    fn from(value: Diagnostic) -> Self {
        Self::Diagnostic(value)
    }
}

impl From<io::Error> for EProverError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

enum ConfiguredOutput<'a, W: Write + ?Sized> {
    Writer(&'a mut W),
    File(File),
}

impl<W: Write + ?Sized> Write for ConfiguredOutput<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self {
            Self::Writer(writer) => writer.write(buffer),
            Self::File(file) => file.write(buffer),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Writer(writer) => writer.flush(),
            Self::File(file) => file.flush(),
        }
    }
}

fn open_configured_output<'a, W: Write + ?Sized>(
    stdout: &'a mut W,
    output_file: Option<&str>,
) -> Result<ConfiguredOutput<'a, W>, Diagnostic> {
    let Some(name) = output_file else {
        return Ok(ConfiguredOutput::Writer(stdout));
    };
    if name == "-" {
        return Ok(ConfiguredOutput::Writer(stdout));
    }

    let path = Path::new(name);
    File::create(path)
        .map(ConfiguredOutput::File)
        .map_err(|error| {
            Diagnostic::new(
                ErrorCode::FILE_ERROR,
                format!("Cannot open file {}: {error}", path.display()),
            )
        })
}

#[allow(clippy::cast_sign_loss)]
const fn c_rlimit_from_arg(value: i64) -> u64 {
    // The C path assigns CLStateGetIntArg()'s signed long to rlim_t.
    value as u64
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

fn apply_time_limit_state(config: &EProverConfig) {
    if let Some(limit) = config.cpu_limit {
        let _ = set_hard_time_limit(c_rlimit_from_arg(limit));
    }
    if let Some(limit) = config.soft_cpu_limit {
        let _ = set_soft_time_limit(c_rlimit_from_arg(limit));
    }
    if let Some(limit) = config.schedule_time_limit {
        let _ = set_schedule_time_limit(c_rlimit_from_arg(limit));
    }
}

const fn memory_limit_bytes_from_mb(memory_mb: i64) -> u64 {
    c_rlimit_from_arg(memory_mb).wrapping_mul(MEGA)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn auto_memory_limit_from_system_mb(system_memory_mb: i64) -> Result<(u64, i64), Diagnostic> {
    if system_memory_mb == -1 {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            "Cannot find physical memory automatically. Give explicit value to --memory-limit",
        ));
    }

    let memory_mb = (system_memory_mb as f64 * 0.8) as i64;
    let delete_bad_limit = (f64::from((c_rlimit_from_arg(memory_mb).wrapping_sub(2)) as f32)
        * 0.7
        * MEGA as f64) as i64;
    Ok((memory_limit_bytes_from_mb(memory_mb), delete_bad_limit))
}

pub fn run<I, S>(
    argv: I,
    stdout: &mut impl Write,
    _stderr: &mut impl Write,
) -> Result<u8, EProverError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match process_options(argv)? {
        EProverAction::Help => {
            stdout.write_all(print_help().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Version => {
            stdout.write_all(version::version_line().as_bytes())?;
            Ok(ErrorCode::NO_ERROR.exit_status())
        }
        EProverAction::Run(config) => run_config(stdout, &config),
    }
}

pub fn process_options<I, S>(argv: I) -> Result<EProverAction, Diagnostic>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut state = CommandLineState::new(argv);
    let mut config = EProverConfig::default();
    while let Some(parsed) = state.next_opt(EPROVER_OPTIONS)? {
        if let Some(action) = apply_parsed_option(&mut config, &parsed)? {
            return Ok(action);
        }
    }

    config.files = state.remaining_args().to_vec();
    if config.files.is_empty() {
        config.files.push("-".to_owned());
    }
    Ok(EProverAction::Run(Box::new(config)))
}

fn apply_parsed_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<Option<EProverAction>, Diagnostic> {
    let option_code = parsed.option().option_code;
    match option_code {
        EProverOption::Help => return Ok(Some(EProverAction::Help)),
        EProverOption::Version => return Ok(Some(EProverAction::Version)),
        _ => {}
    }

    if is_output_option(option_code) {
        apply_output_option(config, parsed)?;
    } else if is_proof_option(option_code) {
        apply_proof_option(config, parsed)?;
    } else if is_resource_option(option_code) {
        apply_resource_option(config, parsed)?;
    } else if is_strategy_option(option_code) {
        apply_strategy_option(config, parsed);
    } else if is_limit_option(option_code) {
        apply_limit_option(config, parsed)?;
    } else if is_format_option(option_code) {
        apply_format_option(config, parsed);
    } else if is_input_mode_option(option_code) {
        apply_input_mode_option(config, parsed);
    } else if is_simple_flag(option_code) {
        apply_simple_flag(config, option_code);
    } else if is_schedule_option(option_code) {
        apply_schedule_option(config, parsed)?;
    } else if is_preprocessing_option(option_code) {
        apply_preprocessing_option(config, parsed)?;
    } else {
        unreachable!("unhandled eprover option");
    }
    Ok(None)
}

const fn is_output_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::Verbose
            | EProverOption::Output
            | EProverOption::Silent
            | EProverOption::OutputLevel
            | EProverOption::PrintStatistics
            | EProverOption::PrintDetailedStatistics
            | EProverOption::PrintSaturated
            | EProverOption::PrintSatInfo
            | EProverOption::FilterSaturated
    )
}

const fn is_proof_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ProofObject
            | EProverOption::ProofGraph
            | EProverOption::ProofStatistics
            | EProverOption::FullDerivation
            | EProverOption::ForceDerivation
            | EProverOption::RecordGivenClauses
            | EProverOption::TrainingExamples
            | EProverOption::PclTermsCompressed
            | EProverOption::PclCompact
            | EProverOption::PclShellLevel
    )
}

const fn is_resource_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::CpuLimit | EProverOption::SoftCpuLimit | EProverOption::MemoryLimit
    )
}

const fn is_strategy_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::SelectStrategy | EProverOption::PrintStrategy | EProverOption::ParseStrategy
    )
}

const fn is_limit_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::ProcessedClausesLimit
            | EProverOption::ProcessedSetLimit
            | EProverOption::UnprocessedLimit
            | EProverOption::TotalClauseSetLimit
            | EProverOption::GeneratedLimit
            | EProverOption::TermBankInsertLimit
            | EProverOption::Answers
    )
}

const fn is_format_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::EqnNoInfix
            | EProverOption::FullEquationalRep
            | EProverOption::PrintOrientedEqLitsAsRules
            | EProverOption::LopIn
            | EProverOption::PclOut
            | EProverOption::TptpIn
            | EProverOption::TptpOut
            | EProverOption::TptpFormat
            | EProverOption::TstpIn
            | EProverOption::TstpOut
            | EProverOption::TstpFormat
    )
}

const fn is_input_mode_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::SyntaxOnly
            | EProverOption::PrintFormulas
            | EProverOption::PruneOnly
            | EProverOption::CnfOnly
    )
}

const fn is_simple_flag(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::PrintPid
            | EProverOption::PrintVersion
            | EProverOption::RequireNonempty
            | EProverOption::ResourcesInfo
            | EProverOption::ConjecturesAreQuestions
            | EProverOption::DeterministicRewriteSort
            | EProverOption::DeterministicNewSort
    )
}

const fn is_schedule_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::Auto
            | EProverOption::AutoSchedule
            | EProverOption::SerializeSchedule
            | EProverOption::ForcePreprocessingSchedule
            | EProverOption::SatAutoSchedule
    )
}

const fn is_preprocessing_option(option: EProverOption) -> bool {
    matches!(
        option,
        EProverOption::NoPreprocessing
            | EProverOption::EqUnfoldLimit
            | EProverOption::EqUnfoldMaxClauses
            | EProverOption::NoEqUnfolding
            | EProverOption::GoalDefs
            | EProverOption::GoalSubtermDefs
            | EProverOption::Sine
            | EProverOption::RelPruningLevel
            | EProverOption::PresatSimplify
            | EProverOption::AcHandling
            | EProverOption::AcNonAggressive
    )
}

fn apply_output_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::Verbose => {
            config.verbose = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::Output => {
            config.output_file = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::Silent => {
            config.output_level = 0;
        }
        EProverOption::OutputLevel => {
            config.output_level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::PrintStatistics => config.flags.set(EProverFlag::PrintStatistics),
        EProverOption::PrintDetailedStatistics => {
            config.flags.set(EProverFlag::PrintDetailedStatistics);
            config.flags.set(EProverFlag::PrintStatistics);
        }
        EProverOption::PrintSaturated => {
            let descriptor = parsed.arg().unwrap_or("").to_owned();
            check_option_letter_string(&descriptor, "teigEIGaA", "-S (--print-saturated)")?;
            config.saturated_output_descriptor = descriptor;
            config.flags.set(EProverFlag::PrintSaturated);
        }
        EProverOption::PrintSatInfo => config.flags.set(EProverFlag::PrintSaturatedInfo),
        EProverOption::FilterSaturated => {
            let descriptor = parsed.arg().unwrap_or("").to_owned();
            check_option_letter_string(&descriptor, "eigEIGaA", "--filter-saturated")?;
            config.filter_saturated_descriptor = descriptor;
            config.flags.set(EProverFlag::FilterSaturated);
        }
        _ => unreachable!("non-output option routed to output handler"),
    }
    Ok(())
}

fn apply_proof_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::ProofObject => {
            let level = get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 3)?;
            config.proof_object_level = config.proof_object_level.max(level);
            config.proof_output = config.proof_output.max(1);
        }
        EProverOption::ProofGraph => {
            let level = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            config.proof_object_level = config.proof_object_level.max(1);
            config.proof_output = level + 1;
        }
        EProverOption::ProofStatistics => config.flags.set(EProverFlag::ProofStatistics),
        EProverOption::FullDerivation => config.flags.set(EProverFlag::FullDerivation),
        EProverOption::ForceDerivation => {
            config.force_derivation_output =
                get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 2)?;
            config.proof_object_level = config.proof_object_level.max(1);
        }
        EProverOption::RecordGivenClauses => {
            config.proof_object_level = config.proof_object_level.max(1);
            config.flags.set(EProverFlag::RecordGivenClauses);
        }
        EProverOption::TrainingExamples => {
            config.proof_object_level = config.proof_object_level.max(1);
            config.flags.set(EProverFlag::RecordGivenClauses);
            config.training_examples =
                Some(get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?);
        }
        EProverOption::PclTermsCompressed => config.pcl_output.full_terms = false,
        EProverOption::PclCompact => config.pcl_output.compact = true,
        EProverOption::PclShellLevel => {
            config.pcl_output.shell_level =
                get_int_arg_check_range(parsed.option(), parsed.arg().unwrap_or(""), 0, 2)?;
        }
        _ => unreachable!("non-proof option routed to proof handler"),
    }
    Ok(())
}

fn apply_resource_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::CpuLimit => {
            let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            if let Some(soft_limit) = config.soft_cpu_limit {
                check_hard_soft_limits(limit, soft_limit, true)?;
            }
            config.cpu_limit = Some(limit);
            config.schedule_time_limit = Some(limit);
        }
        EProverOption::SoftCpuLimit => {
            let limit = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
            if let Some(hard_limit) = config.cpu_limit {
                check_hard_soft_limits(hard_limit, limit, false)?;
            }
            config.soft_cpu_limit = Some(limit);
            config.schedule_time_limit = Some(limit);
        }
        EProverOption::MemoryLimit => {
            let arg = parsed.arg().unwrap_or("");
            if arg == "Auto" {
                let (memory_limit, delete_bad_limit) =
                    auto_memory_limit_from_system_mb(get_system_phys_memory())?;
                config.memory_limit = memory_limit;
                config.delete_bad_limit = delete_bad_limit;
            } else {
                let memory_mb = get_int_arg(parsed.option(), arg)?;
                config.memory_limit = memory_limit_bytes_from_mb(memory_mb);
            }
        }
        _ => unreachable!("non-resource option routed to resource handler"),
    }
    Ok(())
}

fn apply_strategy_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::SelectStrategy => {
            config.select_strategy = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::PrintStrategy => {
            config.print_strategy = Some(parsed.arg().unwrap_or("").to_owned());
        }
        EProverOption::ParseStrategy => {
            config.parse_strategy_file = Some(parsed.arg().unwrap_or("").to_owned());
        }
        _ => unreachable!("non-strategy option routed to strategy handler"),
    }
}

fn schedule_core_arg(parsed: &ParsedOpt<'_, EProverOption>) -> Result<i64, Diagnostic> {
    let arg = parsed.arg().unwrap_or("");
    if arg == "Auto" {
        Ok(-1)
    } else {
        get_int_arg(parsed.option(), arg)
    }
}

fn apply_schedule_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::Auto => {
            if !config.flags.contains(EProverFlag::Auto) {
                config.sine = Some("Auto".to_owned());
                config.flags.set(EProverFlag::Auto);
            }
        }
        EProverOption::AutoSchedule => {
            if !config.strategy_scheduling {
                config.schedule_cores = schedule_core_arg(parsed)?;
                config.sine = Some("Auto".to_owned());
                config.strategy_scheduling = true;
            }
        }
        EProverOption::SerializeSchedule => {
            config.serialize_schedule = get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::ForcePreprocessingSchedule => {
            config.force_preprocessing_schedule =
                get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::SatAutoSchedule => {
            if !config.strategy_scheduling {
                config.schedule_cores = schedule_core_arg(parsed)?;
                config.strategy_scheduling = true;
            }
        }
        _ => unreachable!("non-schedule option routed to schedule handler"),
    }
    Ok(())
}

fn apply_preprocessing_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    match parsed.option().option_code {
        EProverOption::NoPreprocessing => config.preprocessing.no_preprocessing = true,
        EProverOption::EqUnfoldLimit => {
            config.preprocessing.eqdef_incrlimit =
                get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::EqUnfoldMaxClauses => {
            config.preprocessing.eqdef_maxclauses =
                get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::NoEqUnfolding => {
            config.preprocessing.eqdef_incrlimit = i64::MIN;
        }
        EProverOption::GoalDefs => apply_goal_defs(config, parsed.arg().unwrap_or(""))?,
        EProverOption::GoalSubtermDefs => {
            config.preprocessing.goal_definitions.subterms = true;
        }
        EProverOption::Sine => config.sine = Some(parsed.arg().unwrap_or("").to_owned()),
        EProverOption::RelPruningLevel => {
            config.preprocessing.relevance_prune_level =
                get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::PresatSimplify => {
            config.preprocessing.presat_interreduction =
                get_bool_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
        }
        EProverOption::AcHandling => apply_ac_handling(config, parsed.arg().unwrap_or(""))?,
        EProverOption::AcNonAggressive => config.preprocessing.ac_res_aggressive = false,
        _ => unreachable!("non-preprocessing option routed to preprocessing handler"),
    }
    Ok(())
}

fn apply_goal_defs(config: &mut EProverConfig, arg: &str) -> Result<(), Diagnostic> {
    match arg {
        "None" => {
            config.preprocessing.goal_definitions.positive = false;
            config.preprocessing.goal_definitions.negative = false;
            Ok(())
        }
        "All" => {
            config.preprocessing.goal_definitions.positive = true;
            config.preprocessing.goal_definitions.negative = true;
            Ok(())
        }
        "Neg" => {
            config.preprocessing.goal_definitions.positive = false;
            config.preprocessing.goal_definitions.negative = true;
            Ok(())
        }
        _ => Err(Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            "Option --goal-defs accepts only None, All, or Neg",
        )),
    }
}

fn apply_ac_handling(config: &mut EProverConfig, arg: &str) -> Result<(), Diagnostic> {
    config.preprocessing.ac_handling = match arg {
        "None" => AcHandling::None,
        "DiscardAll" => AcHandling::DiscardAll,
        "KeepUnits" => AcHandling::KeepUnits,
        "KeepOrientable" => AcHandling::KeepOrientable,
        _ => {
            return Err(Diagnostic::new(
                ErrorCode::USAGE_ERROR,
                "Option --ac_handling requires None, DiscardAll, KeepUnits, or KeepOrientable as an argument",
            ));
        }
    };
    Ok(())
}

fn apply_limit_option(
    config: &mut EProverConfig,
    parsed: &ParsedOpt<'_, EProverOption>,
) -> Result<(), Diagnostic> {
    let value = get_int_arg(parsed.option(), parsed.arg().unwrap_or(""))?;
    match parsed.option().option_code {
        EProverOption::ProcessedClausesLimit => config.step_limit = value,
        EProverOption::ProcessedSetLimit => config.processed_set_limit = value,
        EProverOption::UnprocessedLimit => config.unprocessed_limit = value,
        EProverOption::TotalClauseSetLimit => config.total_clause_set_limit = value,
        EProverOption::GeneratedLimit => config.generated_limit = value,
        EProverOption::TermBankInsertLimit => config.term_bank_insert_limit = value,
        EProverOption::Answers => config.answer_limit = value,
        _ => unreachable!("non-limit option routed to limit handler"),
    }
    Ok(())
}

fn apply_format_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::EqnNoInfix => config.equation_print.use_infix = false,
        EProverOption::FullEquationalRep => config.equation_print.full_equational_rep = true,
        EProverOption::PrintOrientedEqLitsAsRules => {
            config.equation_print.print_oriented = true;
        }
        EProverOption::LopIn => config.parse_format = IoFormat::Lop,
        EProverOption::PclOut => config.doc_output_format = DocOutputFormat::Pcl,
        EProverOption::TptpIn => config.parse_format = IoFormat::Tptp,
        EProverOption::TptpOut => {
            config.output_format = IoFormat::Tptp;
            config.equation_print.full_equational_rep = false;
            config.equation_print.use_infix = false;
        }
        EProverOption::TptpFormat => {
            config.parse_format = IoFormat::Tptp;
            config.output_format = IoFormat::Tptp;
            config.equation_print.full_equational_rep = false;
            config.equation_print.use_infix = false;
        }
        EProverOption::TstpIn => config.parse_format = IoFormat::Tstp,
        EProverOption::TstpOut => {
            config.doc_output_format = DocOutputFormat::Tstp;
            config.output_format = IoFormat::Tstp;
            config.equation_print.use_infix = true;
        }
        EProverOption::TstpFormat => {
            config.parse_format = IoFormat::Tstp;
            config.doc_output_format = DocOutputFormat::Tstp;
            config.output_format = IoFormat::Tstp;
            config.equation_print.use_infix = true;
        }
        _ => unreachable!("non-format option routed to format handler"),
    }
}

fn apply_input_mode_option(config: &mut EProverConfig, parsed: &ParsedOpt<'_, EProverOption>) {
    match parsed.option().option_code {
        EProverOption::SyntaxOnly => config.flags.set(EProverFlag::SyntaxOnly),
        EProverOption::PrintFormulas => {
            config.flags.set(EProverFlag::SyntaxOnly);
            config.flags.set(EProverFlag::PrintFormulas);
        }
        EProverOption::PruneOnly => {
            config.output_level = 4;
            config.flags.set(EProverFlag::PruneOnly);
        }
        EProverOption::CnfOnly => {
            "teigEIG".clone_into(&mut config.saturated_output_descriptor);
            config.step_limit = 0;
            config.flags.set(EProverFlag::PrintSaturated);
            config.flags.set(EProverFlag::CnfOnly);
        }
        _ => unreachable!("non-input-mode option routed to input-mode handler"),
    }
}

fn apply_simple_flag(config: &mut EProverConfig, option: EProverOption) {
    match option {
        EProverOption::PrintPid => config.flags.set(EProverFlag::PrintPid),
        EProverOption::PrintVersion => config.flags.set(EProverFlag::PrintVersion),
        EProverOption::RequireNonempty => config.flags.set(EProverFlag::RequireNonempty),
        EProverOption::ResourcesInfo => config.flags.set(EProverFlag::ResourceInfo),
        EProverOption::ConjecturesAreQuestions => {
            config.flags.set(EProverFlag::ConjecturesAreQuestions);
        }
        EProverOption::DeterministicRewriteSort => {
            config.flags.set(EProverFlag::DeterministicRewriteSort);
        }
        EProverOption::DeterministicNewSort => {
            config.flags.set(EProverFlag::DeterministicNewSort);
        }
        _ => unreachable!("non-simple flag routed to simple-flag handler"),
    }
}

#[must_use]
pub fn print_help() -> String {
    let mut result = format!(
        "\nE {VERSION} \"{E_NICKNAME}\"\n\n\
Usage: {PROGRAM_NAME} [options] [files]\n\n\
Read a set of first-order (or, in the -ho-version, higher-order)\n\
clauses and formulae and try to prove the conjecture (if given)\n\
or show the set unsatisfiable.\n\n"
    );
    result.push_str(&print_options(EPROVER_OPTIONS, Some("Options:\n\n")));
    result.push_str("\n\n");
    result.push_str(&version::footer());
    result
}

fn run_config(stdout: &mut impl Write, config: &EProverConfig) -> Result<u8, EProverError> {
    let verbose = i32::try_from(config.verbose).map_err(|_| {
        Diagnostic::new(
            ErrorCode::USAGE_ERROR,
            format!("--verbose argument {} is out of int range", config.verbose),
        )
    })?;
    set_verbose_level(verbose);
    let _ = set_output_level(config.output_level);
    apply_time_limit_state(config);
    let _ = set_memory_limit(config.memory_limit);
    let mut output = open_configured_output(stdout, config.output_file.as_deref())?;

    if config.flags.contains(EProverFlag::PrintPid) {
        writeln!(output, "# Pid: {}", std::process::id())?;
    }
    if config.flags.contains(EProverFlag::PrintVersion) {
        writeln!(output, "# Version: {VERSION}")?;
    }
    output.flush()?;

    Err(Diagnostic::new(
        ErrorCode::OTHER_ERROR,
        "Rust eprover proof search is not implemented yet",
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::{
        auto_memory_limit_from_system_mb, process_options, run, AcHandling, DocOutputFormat,
        EProverAction, EProverFlag, MEGA,
    };
    use crate::basics::error::ErrorCode;
    use crate::basics::verbose::{set_verbose_level, verbose_level};
    use crate::inout::output::{output_level, set_output_level};
    use crate::inout::scanner::IoFormat;
    use crate::inout::signals::{
        hard_time_limit, schedule_time_limit, set_hard_time_limit, set_schedule_time_limit,
        set_soft_time_limit, soft_time_limit, RLIM_INFINITY_COMPAT,
    };
    use crate::prover::version::VERSION;
    use std::sync::{Mutex, OnceLock};

    fn global_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("eprover-{name}-{}.out", std::process::id()))
    }

    #[test]
    fn process_options_recognizes_version_action() {
        let action = process_options(["eprover", "--version"]).unwrap();
        assert_eq!(action, EProverAction::Version);
    }

    #[test]
    fn process_options_keeps_non_option_files_and_inserts_stdin_default() {
        let action = process_options(["eprover", "a.p", "--silent", "b.p"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_level, 0);
        assert_eq!(config.files, ["a.p", "b.p"]);

        let action = process_options(["eprover", "--silent"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.files, ["-"]);
    }

    #[test]
    fn process_options_tracks_cpu_limit_schedule_state_like_c() {
        let action = process_options(["eprover", "--cpu-limit"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(300));
        assert_eq!(config.soft_cpu_limit, None);
        assert_eq!(config.schedule_time_limit, Some(300));

        let action =
            process_options(["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.cpu_limit, Some(100));
        assert_eq!(config.soft_cpu_limit, Some(25));
        assert_eq!(config.schedule_time_limit, Some(100));

        let action =
            process_options(["eprover", "--cpu-limit=100", "--soft-cpu-limit=25"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.schedule_time_limit, Some(25));
    }

    #[test]
    fn process_options_records_memory_limit_state_like_c() {
        let action = process_options(["eprover", "-m", "128"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.memory_limit, 128 * MEGA);
        assert_eq!(config.delete_bad_limit, i64::MAX);

        let action = process_options(["eprover", "--memory-limit=-1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.memory_limit, u64::MAX.wrapping_mul(MEGA));
    }

    #[test]
    fn auto_memory_limit_derives_search_limits_like_c() {
        let (memory_limit, delete_bad_limit) = auto_memory_limit_from_system_mb(1_000).unwrap();

        assert_eq!(memory_limit, 800 * MEGA);
        assert_eq!(delete_bad_limit, 585_734_553);

        let error = auto_memory_limit_from_system_mb(-1).unwrap_err();
        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            error.message(),
            "Cannot find physical memory automatically. Give explicit value to --memory-limit"
        );
    }

    #[test]
    fn process_options_records_input_mode_flags_like_c() {
        let action = process_options(["eprover", "--print-formulas"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::SyntaxOnly));
        assert!(config.flags.contains(EProverFlag::PrintFormulas));

        let action = process_options(["eprover", "--prune"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_level, 4);
        assert!(config.flags.contains(EProverFlag::PruneOnly));

        let action = process_options(["eprover", "--cnf", "--error-on-empty"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::CnfOnly));
        assert!(config.flags.contains(EProverFlag::PrintSaturated));
        assert!(config.flags.contains(EProverFlag::RequireNonempty));
        assert_eq!(config.saturated_output_descriptor, "teigEIG");
        assert_eq!(config.step_limit, 0);
    }

    #[test]
    fn process_options_records_reporting_descriptors_like_c() {
        let action = process_options([
            "eprover",
            "--print-statistics",
            "--print-detailed-statistics",
            "-S",
            "--filter-saturated=eig",
            "--print-sat-info",
            "-R",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::PrintStatistics));
        assert!(config.flags.contains(EProverFlag::PrintDetailedStatistics));
        assert!(config.flags.contains(EProverFlag::PrintSaturated));
        assert!(config.flags.contains(EProverFlag::FilterSaturated));
        assert!(config.flags.contains(EProverFlag::PrintSaturatedInfo));
        assert!(config.flags.contains(EProverFlag::ResourceInfo));
        assert_eq!(config.saturated_output_descriptor, "eigEIG");
        assert_eq!(config.filter_saturated_descriptor, "eig");

        let action =
            process_options(["eprover", "--print-saturated=teA", "--filter-saturated=eig"])
                .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.saturated_output_descriptor, "teA");
        assert_eq!(config.filter_saturated_descriptor, "eig");
    }

    #[test]
    fn process_options_rejects_invalid_reporting_descriptors() {
        let error = process_options(["eprover", "--print-saturated=tx"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option -S (--print-saturated)"
        );

        let error = process_options(["eprover", "--filter-saturated"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option --filter-saturated"
        );

        let error = process_options(["eprover", "--filter-saturated=Fx"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Illegal argument to option --filter-saturated"
        );
    }

    #[test]
    fn process_options_records_strategy_and_limit_state_like_c() {
        let action = process_options([
            "eprover",
            "--select-strategy=AutoSched",
            "--print-strategy",
            "--parse-strategy=strategy.txt",
            "-C",
            "10",
            "-P",
            "20",
            "-U",
            "30",
            "-T",
            "40",
            "--generated-limit=50",
            "--tb-insert-limit=60",
            "--answers",
            "--conjectures-are-questions",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.select_strategy.as_deref(), Some("AutoSched"));
        assert_eq!(config.print_strategy.as_deref(), Some(">current-strategy<"));
        assert_eq!(config.parse_strategy_file.as_deref(), Some("strategy.txt"));
        assert_eq!(config.step_limit, 10);
        assert_eq!(config.processed_set_limit, 20);
        assert_eq!(config.unprocessed_limit, 30);
        assert_eq!(config.total_clause_set_limit, 40);
        assert_eq!(config.generated_limit, 50);
        assert_eq!(config.term_bank_insert_limit, 60);
        assert_eq!(config.answer_limit, 2_147_483_647);
        assert!(config.flags.contains(EProverFlag::ConjecturesAreQuestions));

        let action = process_options(["eprover", "--answers=7", "--print-strategy=Named"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.answer_limit, 7);
        assert_eq!(config.print_strategy.as_deref(), Some("Named"));
    }

    #[test]
    fn process_options_records_format_state_like_c() {
        let action = process_options([
            "eprover",
            "--lop-in",
            "--pcl-out",
            "--pcl-terms-compressed",
            "--pcl-compact",
            "--pcl-shell-level",
            "--print-oriented-eqlits-as-rules",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.parse_format, IoFormat::Lop);
        assert_eq!(config.output_format, IoFormat::Lop);
        assert_eq!(config.doc_output_format, DocOutputFormat::Pcl);
        assert!(!config.pcl_output.full_terms);
        assert!(config.pcl_output.compact);
        assert_eq!(config.pcl_output.shell_level, 1);
        assert!(config.equation_print.print_oriented);

        let action = process_options(["eprover", "--pcl-shell-level=2"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.pcl_output.shell_level, 2);
    }

    #[test]
    fn process_options_records_tptp_and_tstp_format_side_effects_like_c() {
        let action = process_options(["eprover", "--full-equational-rep", "--tptp-out"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_format, IoFormat::Tptp);
        assert!(!config.equation_print.full_equational_rep);
        assert!(!config.equation_print.use_infix);

        let action = process_options(["eprover", "--tptp2-format"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.parse_format, IoFormat::Tptp);
        assert_eq!(config.output_format, IoFormat::Tptp);
        assert!(!config.equation_print.use_infix);

        let action = process_options([
            "eprover",
            "--eqn-no-infix",
            "--full-equational-rep",
            "--tstp-out",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.output_format, IoFormat::Tstp);
        assert_eq!(config.doc_output_format, DocOutputFormat::Tstp);
        assert!(config.equation_print.use_infix);
        assert!(config.equation_print.full_equational_rep);

        let action = process_options(["eprover", "--tptp3-format"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.parse_format, IoFormat::Tstp);
        assert_eq!(config.output_format, IoFormat::Tstp);
        assert_eq!(config.doc_output_format, DocOutputFormat::Tstp);
    }

    #[test]
    fn process_options_records_auto_schedule_state_like_c() {
        let action = process_options(["eprover", "--auto"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::Auto));
        assert_eq!(config.sine.as_deref(), Some("Auto"));
        assert!(!config.strategy_scheduling);

        let action = process_options([
            "eprover",
            "--auto-schedule=Auto",
            "--serialize-schedule=true",
            "--force-preproc-sched=false",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.strategy_scheduling);
        assert_eq!(config.schedule_cores, -1);
        assert_eq!(config.sine.as_deref(), Some("Auto"));
        assert!(config.serialize_schedule);
        assert!(!config.force_preprocessing_schedule);

        let action =
            process_options(["eprover", "--satauto-schedule", "--auto-schedule=4"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.strategy_scheduling);
        assert_eq!(config.schedule_cores, 1);
        assert_eq!(config.sine, None);

        let action =
            process_options(["eprover", "--auto-schedule=4", "--satauto-schedule=8"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.schedule_cores, 4);
        assert_eq!(config.sine.as_deref(), Some("Auto"));
    }

    #[test]
    fn process_options_rejects_invalid_schedule_bool_args() {
        let error = process_options(["eprover", "--serialize-schedule=yes"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);

        let error = process_options(["eprover", "--force-preproc-sched=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_records_preprocessing_state_like_c() {
        let action = process_options(["eprover"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.preprocessing.no_preprocessing);
        assert_eq!(config.preprocessing.eqdef_incrlimit, 20);
        assert_eq!(config.preprocessing.eqdef_maxclauses, 20_000);
        assert_eq!(config.preprocessing.ac_handling, AcHandling::DiscardAll);
        assert!(config.preprocessing.ac_res_aggressive);
        assert!(!config.preprocessing.presat_interreduction);
        assert_eq!(config.preprocessing.relevance_prune_level, 0);

        let action = process_options([
            "eprover",
            "--no-preprocessing",
            "--eq-unfold-limit=7",
            "--eq-unfold-maxclauses=11",
            "--goal-defs",
            "--goal-subterm-defs",
            "--sine",
            "--rel-pruning-level",
            "--presat-simplify=false",
            "--ac-handling=KeepOrientable",
            "--ac-non-aggressive",
        ])
        .unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.preprocessing.no_preprocessing);
        assert_eq!(config.preprocessing.eqdef_incrlimit, 7);
        assert_eq!(config.preprocessing.eqdef_maxclauses, 11);
        assert!(config.preprocessing.goal_definitions.positive);
        assert!(config.preprocessing.goal_definitions.negative);
        assert!(config.preprocessing.goal_definitions.subterms);
        assert_eq!(config.sine.as_deref(), Some("Auto"));
        assert_eq!(config.preprocessing.relevance_prune_level, 3);
        assert!(!config.preprocessing.presat_interreduction);
        assert_eq!(config.preprocessing.ac_handling, AcHandling::KeepOrientable);
        assert!(!config.preprocessing.ac_res_aggressive);

        let action = process_options(["eprover", "--no-eq-unfolding", "--ac-handling"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.preprocessing.eqdef_incrlimit, i64::MIN);
        assert_eq!(config.preprocessing.ac_handling, AcHandling::KeepUnits);
    }

    #[test]
    fn process_options_records_goal_defs_modes_like_c() {
        let action = process_options(["eprover", "--goal-defs=None"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.preprocessing.goal_definitions.positive);
        assert!(!config.preprocessing.goal_definitions.negative);

        let action = process_options(["eprover", "--goal-defs=Neg"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(!config.preprocessing.goal_definitions.positive);
        assert!(config.preprocessing.goal_definitions.negative);

        let action = process_options(["eprover", "--goal-defs=All"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.preprocessing.goal_definitions.positive);
        assert!(config.preprocessing.goal_definitions.negative);
    }

    #[test]
    fn process_options_rejects_invalid_preprocessing_modes() {
        let error = process_options(["eprover", "--goal-defs=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --goal-defs accepts only None, All, or Neg"
        );

        let error = process_options(["eprover", "--ac-handling=Bad"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Option --ac_handling requires None, DiscardAll, KeepUnits, or KeepOrientable as an argument"
        );

        let error = process_options(["eprover", "--presat-simplify=maybe"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_rejects_invalid_pcl_shell_level() {
        let error = process_options(["eprover", "--pcl-shell-level=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_records_proof_output_state_like_c() {
        let action = process_options(["eprover", "--proof-object=3", "--proof-object=1"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 3);
        assert_eq!(config.proof_output, 1);

        let action = process_options(["eprover", "--proof-graph=2"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.proof_object_level, 1);
        assert_eq!(config.proof_output, 3);

        let action =
            process_options(["eprover", "-d", "--proof-statistics", "--record-gcs"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert!(config.flags.contains(EProverFlag::FullDerivation));
        assert!(config.flags.contains(EProverFlag::ProofStatistics));
        assert!(config.flags.contains(EProverFlag::RecordGivenClauses));
        assert_eq!(config.proof_object_level, 1);

        let action =
            process_options(["eprover", "--force-deriv=2", "--training-examples=3"]).unwrap();
        let EProverAction::Run(config) = action else {
            panic!("expected run config");
        };
        assert_eq!(config.force_derivation_output, 2);
        assert_eq!(config.training_examples, Some(3));
        assert!(config.flags.contains(EProverFlag::RecordGivenClauses));
        assert_eq!(config.proof_object_level, 1);
    }

    #[test]
    fn process_options_rejects_force_derivation_outside_c_range() {
        let error = process_options(["eprover", "--force-deriv=3"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
    }

    #[test]
    fn process_options_rejects_non_increasing_cpu_limits() {
        let error =
            process_options(["eprover", "--cpu-limit=10", "--soft-cpu-limit=10"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Soft time limit has to be smaller than hardtime limit"
        );

        let error =
            process_options(["eprover", "--soft-cpu-limit=20", "--cpu-limit=10"]).unwrap_err();
        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert_eq!(
            error.message(),
            "Hard time limit has to be larger than softtime limit"
        );
    }

    #[test]
    fn run_version_prints_c_compatible_version_line() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(["eprover", "-V"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "E 3.3.5 Countess Grey (facc36eaf92d70896d830140efc4382df9e8dcdb)\n"
        );
        assert!(stderr.is_empty());
    }

    #[test]
    fn run_help_prints_usage() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = run(["eprover", "-h"], &mut stdout, &mut stderr).unwrap();
        assert_eq!(status, ErrorCode::NO_ERROR.exit_status());
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("Usage: eprover [options] [files]"));
        assert!(output.contains("--version"));
    }

    #[test]
    fn run_applies_verbose_option_to_global_gate() {
        let _guard = global_test_lock();
        set_verbose_level(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", "--verbose=2"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(verbose_level(), 2);
        set_verbose_level(0);
    }

    #[test]
    fn run_rejects_verbose_values_outside_c_int_range() {
        let _guard = global_test_lock();
        set_verbose_level(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--verbose=2147483648"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::USAGE_ERROR);
        assert!(error.message().contains("out of int range"));
        assert_eq!(verbose_level(), 0);
    }

    #[test]
    fn run_applies_cpu_limit_options_to_signal_state() {
        let _guard = global_test_lock();
        let _ = set_hard_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_soft_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_schedule_time_limit(0);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--soft-cpu-limit=25", "--cpu-limit=100"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(hard_time_limit(), 100);
        assert_eq!(soft_time_limit(), 25);
        assert_eq!(schedule_time_limit(), 100);

        let _ = set_hard_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_soft_time_limit(RLIM_INFINITY_COMPAT);
        let _ = set_schedule_time_limit(0);
    }

    #[test]
    fn run_applies_output_level_options_to_global_gate() {
        let _guard = global_test_lock();
        let _ = set_output_level(1);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(["eprover", "--silent"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(output_level(), 0);

        let error = run(["eprover", "--output-level=3"], &mut stdout, &mut stderr).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(output_level(), 3);
        let _ = set_output_level(1);
    }

    #[test]
    fn run_print_info_uses_configured_output_target() {
        let _guard = global_test_lock();
        let path = temp_path("print-info");
        let _ = std::fs::remove_file(&path);
        let path_arg = path.to_string_lossy().into_owned();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let error = run(
            ["eprover", "--print-version", "-o", path_arg.as_str()],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert!(stdout.is_empty());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("# Version: {VERSION}\n")
        );
        std::fs::remove_file(&path).unwrap();

        let error = run(
            ["eprover", "--print-version", "-o", "-"],
            &mut stdout,
            &mut stderr,
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            format!("# Version: {VERSION}\n")
        );
    }
}
