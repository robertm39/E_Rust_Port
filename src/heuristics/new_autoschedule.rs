//! Predefined strategy lookup from C `che_new_autoschedule`.

use crate::basics::error::{Diagnostic, ErrorCode};
use crate::heuristics::hcb::{
    heuristic_parms_parse_into, heuristic_parms_print_string, HeuristicParmsCell,
};
use crate::heuristics::to_params::TermOrdering;
use crate::inout::scanner::{Scanner, TokenType};

#[cfg(test)]
const SCHEDULE_VARS: &str = include_str!("schedule.vars");
pub const DEFAULT_MASK: &str = "aaaaa-aaaaaa-aaaaaaaaa";
pub const DEFAULT_SCHED_TIME_LIMIT: u64 = 300;
pub const SCHEDULE_DONE: i32 = -1;
pub const RETRY_DEFAULT_SCHEDULE_THRESHOLD: f64 = 2.0;

const PLACEHOLDER_STRATEGY: &str = "<placeholder>";
const PREPROCESSING_INSERT_RATIO: f64 = 0.1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PredefinedStrategy {
    name: &'static str,
    definition: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct StaticScheduleCell {
    heuristic_name: &'static str,
    ordering: TermOrdering,
    sine: Option<&'static str>,
    time_fraction: f64,
    time_absolute: u64,
    cores: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleCell {
    pub heuristic_name: String,
    pub ordering: TermOrdering,
    pub sine: Option<String>,
    pub time_fraction: f64,
    pub time_absolute: u64,
    pub cores: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScheduleClass {
    key: &'static str,
    schedule: &'static [StaticScheduleCell],
    class_size: i32,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct StaticNamedSchedule {
    name: &'static str,
    cells: &'static [StaticScheduleCell],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSchedule {
    pub matched_class: String,
    pub distance: usize,
    pub class_size: i32,
    pub schedule: Vec<ScheduleCell>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduleMultiCoreInitReport {
    pub scheduled: usize,
    pub cores: i32,
    pub limit: u64,
    pub total_time: u64,
}

include!(concat!(env!("OUT_DIR"), "/schedule_tables.rs"));

/// Parses a named predefined strategy into `target`, matching C
/// `GetHeuristicWithName`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded C schedule table cannot be parsed, the
/// requested name is not present, or the selected parameter block is malformed.
pub fn get_heuristic_with_name(
    name: &str,
    target: &mut HeuristicParmsCell,
) -> Result<(), Diagnostic> {
    let Some(definition) = predefined_strategy_definition(name) else {
        return Err(Diagnostic::new(
            ErrorCode::OTHER_ERROR,
            format!("Error: Configuration name {name} not found."),
        ));
    };
    let mut scanner = Scanner::from_internal_string(definition, true)?;
    heuristic_parms_parse_into(&mut scanner, target, false)?;
    scanner.check_tok(TokenType::NO_TOKEN)
}

/// Renders predefined strategies like C `StrategiesPrintPredefined`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded C schedule table cannot be parsed.
pub fn strategies_print_predefined_string(names_only: bool) -> Result<String, Diagnostic> {
    let mut result = String::new();
    for strategy in PREDEFINED_STRATEGIES {
        if names_only {
            result.push_str(strategy.name);
            result.push('\n');
        } else {
            result.push_str(strategy.name);
            result.push_str(" = \n");
            result.push_str(strategy.definition);
            result.push('\n');
        }
    }
    Ok(result)
}

/// Returns C `GetPreprocessingSchedule(problem_category)`.
///
/// The returned `ResolvedSchedule` includes the selected class metadata so
/// callers can reproduce C's partial-match comment when `distance != 0`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded generated schedule table cannot be
/// parsed or references an unknown schedule array.
pub fn get_preprocessing_schedule(problem_category: &str) -> Result<ResolvedSchedule, Diagnostic> {
    resolve_schedule(problem_category, PREPROCESSING_MAP)
}

/// Returns C `GetSearchSchedule(problem_category)`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded generated schedule table cannot be
/// parsed or references an unknown schedule array.
pub fn get_search_schedule(problem_category: &str) -> Result<ResolvedSchedule, Diagnostic> {
    resolve_schedule(problem_category, SEARCH_MAP)
}

/// Returns C `GetDefaultSchedule()`.
///
/// # Errors
///
/// Returns a diagnostic if the embedded generated schedule table cannot be
/// parsed or the default schedule array is missing.
pub fn get_default_schedule() -> Result<Vec<ScheduleCell>, Diagnostic> {
    Ok(owned_schedule(DEFAULT_SCHEDULE))
}

/// C `StrDistance`: positional character mismatches plus length difference.
#[must_use]
pub fn schedule_string_distance(left: &str, right: &str) -> usize {
    let mut distance = 0_usize;
    let mut left_chars = left.chars();
    let mut right_chars = right.chars();
    loop {
        match (left_chars.next(), right_chars.next()) {
            (Some(left_char), Some(right_char)) => {
                distance += usize::from(left_char != right_char);
            }
            (Some(_), None) => {
                distance += 1 + left_chars.count();
                break;
            }
            (None, Some(_)) => {
                distance += 1 + right_chars.count();
                break;
            }
            (None, None) => break,
        }
    }
    distance
}

/// C `ScheduleTimesInit`: initialize per-strategy absolute time limits.
///
/// `schedule_time_limit` corresponds to the C global `ScheduleTimeLimit`; when
/// it is absent or zero, C uses [`DEFAULT_SCHED_TIME_LIMIT`] for all but the
/// final strategy and gives the final strategy `RLIM_INFINITY`.
pub fn schedule_times_init(
    schedule: &mut [ScheduleCell],
    time_used: f64,
    schedule_time_limit: Option<u64>,
) {
    if schedule.is_empty() {
        return;
    }

    let configured_limit = schedule_time_limit.unwrap_or(0);
    let limit = if configured_limit != 0 {
        remaining_time(f64_from_u64(configured_limit), time_used)
    } else {
        remaining_time(f64_from_u64(DEFAULT_SCHED_TIME_LIMIT), time_used)
    };

    let mut sum = 0_u64;
    let last_index = schedule.len() - 1;
    for cell in schedule.iter_mut().take(last_index) {
        let time = trunc_to_u64_saturating(cell.time_fraction * f64_from_u64(limit));
        cell.time_absolute = time;
        sum = sum.saturating_add(time);
    }

    if configured_limit != 0 {
        schedule[last_index].time_absolute = limit.saturating_sub(sum);
    } else {
        schedule[last_index].time_absolute = u64::MAX;
    }
}

/// C `ScheduleTimesInitMultiCore`: initialize absolute limits and core counts.
///
/// This pure helper mutates a per-run owned schedule copy. It returns the values
/// C prints in its scheduling comment so the executable can preserve that
/// output when process execution is wired.
pub fn schedule_times_init_multi_core(
    schedule: &mut Vec<ScheduleCell>,
    time_used: f64,
    time_limit: f64,
    preprocessing_schedule: bool,
    cores: &mut i32,
    serialize: bool,
) -> ScheduleMultiCoreInitReport {
    let mut schedule_size = schedule.len();
    if preprocessing_schedule && schedule_size > usize_from_nonnegative_i32(*cores) {
        schedule_size = usize_from_nonnegative_i32(*cores);
        schedule.truncate(schedule_size);
        rescale_schedule_fractions(schedule);
    }

    let limit = ceil_to_u64_saturating(time_limit - time_used);
    let total_limit = limit;
    let mut allocated_cores = 0_i32;

    if preprocessing_schedule {
        if serialize {
            for cell in schedule.iter_mut() {
                cell.cores = 1;
            }
            *cores = 1;
        } else {
            for cell in schedule.iter_mut() {
                cell.cores = ceil_to_i32_min_one(cell.time_fraction * f64::from(*cores));
                allocated_cores += cell.cores;
            }
            let mut error = allocated_cores - *cores;
            debug_assert!(usize_from_nonnegative_i32(error) <= schedule_size);
            for cell in schedule.iter_mut().rev() {
                if error == 0 {
                    break;
                }
                let to_take = (cell.cores - 1).min(error);
                cell.cores -= to_take;
                error -= to_take;
            }
            debug_assert_eq!(error, 0);
        }
    }

    let mut sum = 0_u64;
    let mut scheduled = 0_usize;
    for cell in schedule.iter_mut() {
        if !preprocessing_schedule && sum >= total_limit {
            break;
        }

        let ratio = if preprocessing_schedule && !serialize {
            1.0
        } else {
            cell.time_fraction
        };
        let raw_time =
            ceil_to_u64_saturating(ratio * f64::from(cell.cores) * f64_from_u64(total_limit));
        let time = if preprocessing_schedule {
            raw_time
        } else {
            raw_time.min(limit.saturating_sub(sum))
        };
        cell.time_absolute = time;
        sum = sum.saturating_add(time);
        scheduled += 1;
    }
    schedule.truncate(scheduled);

    ScheduleMultiCoreInitReport {
        scheduled,
        cores: *cores,
        limit,
        total_time: sum,
    }
}

/// C `InitializePlaceholderSearchSchedule`.
///
/// # Errors
///
/// Returns a diagnostic if the search schedule lacks the generated placeholder
/// entry or if forced insertion is requested with an empty preprocessing
/// schedule.
pub fn initialize_placeholder_search_schedule(
    search_schedule: &mut Vec<ScheduleCell>,
    preprocessing_schedule: &[ScheduleCell],
    mut force_preprocessing: bool,
) -> Result<(), Diagnostic> {
    let placeholder_index = search_schedule
        .iter()
        .position(|cell| cell.heuristic_name == PLACEHOLDER_STRATEGY)
        .ok_or_else(|| schedule_parse_error("Search schedule lacks placeholder entry"))?;

    if force_preprocessing {
        let preprocessing_name = preprocessing_schedule
            .first()
            .map(|cell| cell.heuristic_name.as_str())
            .ok_or_else(|| {
                schedule_parse_error("Forced preprocessing schedule insertion needs a schedule")
            })?;
        if search_schedule
            .iter()
            .take(placeholder_index)
            .any(|cell| cell.heuristic_name == preprocessing_name)
        {
            force_preprocessing = false;
        }
    }

    if !force_preprocessing {
        search_schedule.truncate(placeholder_index);
        return Ok(());
    }

    let preprocessing_name = preprocessing_schedule[0].heuristic_name.clone();
    search_schedule[placeholder_index].heuristic_name = preprocessing_name;
    search_schedule[placeholder_index].time_fraction = PREPROCESSING_INSERT_RATIO;
    for cell in search_schedule.iter_mut().take(placeholder_index) {
        cell.time_fraction *= 1.0 - PREPROCESSING_INSERT_RATIO;
    }
    search_schedule.swap(placeholder_index, 1);
    Ok(())
}

/// C `GetFilteredDefaultSchedule` over caller-owned schedule copies.
#[must_use]
pub fn get_filtered_default_schedule(
    default_schedule: &[ScheduleCell],
    exhausted_schedule: &[ScheduleCell],
) -> Vec<ScheduleCell> {
    let mut filtered = default_schedule
        .iter()
        .filter(|cell| !name_in_schedule(&cell.heuristic_name, exhausted_schedule))
        .cloned()
        .collect::<Vec<_>>();

    if let Some(last_index) = filtered.len().checked_sub(1) {
        let ratio = 1.0 / f64_from_usize(filtered.len());
        for cell in filtered.iter_mut().take(last_index) {
            cell.time_fraction = ratio;
        }
    }

    filtered
}

fn predefined_strategy_definition(name: &str) -> Option<&'static str> {
    PREDEFINED_STRATEGIES
        .iter()
        .find(|strategy| strategy.name == name)
        .map(|strategy| strategy.definition)
}

fn schedule_parse_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::OTHER_ERROR, message)
}

fn owned_schedule(schedule: &[StaticScheduleCell]) -> Vec<ScheduleCell> {
    schedule
        .iter()
        .map(|cell| ScheduleCell {
            heuristic_name: cell.heuristic_name.to_owned(),
            ordering: cell.ordering,
            sine: cell.sine.map(str::to_owned),
            time_fraction: cell.time_fraction,
            time_absolute: cell.time_absolute,
            cores: cell.cores,
        })
        .collect()
}

fn remaining_time(limit: f64, time_used: f64) -> u64 {
    if limit > time_used {
        trunc_to_u64_saturating(limit - time_used)
    } else {
        0
    }
}

fn rescale_schedule_fractions(schedule: &mut [ScheduleCell]) {
    let total_ratio = schedule.iter().map(|cell| cell.time_fraction).sum::<f64>();
    if total_ratio == 0.0 {
        return;
    }
    let factor = 1.0 / total_ratio;
    for cell in schedule {
        cell.time_fraction *= factor;
    }
}

fn name_in_schedule(name: &str, schedule: &[ScheduleCell]) -> bool {
    schedule.iter().any(|cell| cell.heuristic_name == name)
}

#[allow(clippy::cast_precision_loss)]
fn f64_from_u64(value: u64) -> f64 {
    value as f64
}

#[allow(clippy::cast_precision_loss)]
fn f64_from_usize(value: usize) -> f64 {
    value as f64
}

#[allow(clippy::cast_possible_truncation)]
fn ceil_to_i32_min_one(value: f64) -> i32 {
    value.ceil().max(1.0) as i32
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn trunc_to_u64_saturating(value: f64) -> u64 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= u64::MAX as f64 {
        u64::MAX
    } else {
        value as u64
    }
}

fn ceil_to_u64_saturating(value: f64) -> u64 {
    trunc_to_u64_saturating(value.ceil())
}

fn usize_from_nonnegative_i32(value: i32) -> usize {
    usize::try_from(value.max(0)).unwrap_or(usize::MAX)
}

fn resolve_schedule(
    problem_category: &str,
    entries: &[ScheduleClass],
) -> Result<ResolvedSchedule, Diagnostic> {
    let (entry, distance) = select_schedule_class(problem_category, entries)
        .ok_or_else(|| Diagnostic::new(ErrorCode::OTHER_ERROR, "Schedule class map is empty"))?;

    Ok(ResolvedSchedule {
        matched_class: entry.key.to_owned(),
        distance,
        class_size: entry.class_size,
        schedule: owned_schedule(entry.schedule),
    })
}

fn select_schedule_class<'a>(
    problem_category: &str,
    entries: &'a [ScheduleClass],
) -> Option<(&'a ScheduleClass, usize)> {
    let mut selected = None;
    let mut min_distance = usize::MAX;
    let mut max_class_size = i32::MIN;

    for entry in entries {
        let distance = schedule_string_distance(entry.key, problem_category);
        if distance == 0 {
            return Some((entry, distance));
        }
        if distance < min_distance
            || (distance == min_distance && entry.class_size > max_class_size)
        {
            selected = Some(entry);
            min_distance = distance;
            max_class_size = entry.class_size;
        }
    }

    selected.map(|entry| (entry, min_distance))
}

/// Prints a single heuristic parameter block.
#[must_use]
pub fn heuristic_parms_strategy_print_string(handle: &HeuristicParmsCell) -> String {
    heuristic_parms_print_string(handle)
}

#[cfg(test)]
mod tests {
    use super::{
        get_default_schedule, get_filtered_default_schedule, get_heuristic_with_name,
        get_preprocessing_schedule, get_search_schedule, initialize_placeholder_search_schedule,
        schedule_string_distance, schedule_times_init, schedule_times_init_multi_core,
        select_schedule_class, strategies_print_predefined_string, ScheduleCell, ScheduleClass,
        StaticScheduleCell, DEFAULT_SCHED_TIME_LIMIT, GENERATED_PREPROCESSING_SCHEDULE_NAMES,
        GENERATED_SCHEDULES, GENERATED_SEARCH_SCHEDULE_NAMES, PREDEFINED_STRATEGIES,
        PREPROCESSING_MAP, SCHEDULE_VARS, SEARCH_MAP,
    };
    use crate::basics::error::ErrorCode;
    use crate::heuristics::hcb::HeuristicParmsCell;
    use crate::heuristics::schedule_vars_parser::{
        parse_schedule_vars, ParsedSchedule, ParsedScheduleClass,
    };
    use crate::heuristics::to_params::TermOrdering;
    use crate::terms::termtypes::RewriteLevel;

    const FIRST_STRATEGY: &str = "G-E--_208_C12_11_nc_F1_SE_CS_SP_PS_S5PRR_S04BN";

    #[test]
    fn predefined_strategy_table_reads_conf_map_only() {
        let names = PREDEFINED_STRATEGIES
            .iter()
            .map(|strategy| strategy.name)
            .collect::<Vec<_>>();

        assert_eq!(names.first().copied(), Some(FIRST_STRATEGY));
        assert!(names.len() > 400);
        assert!(!names.contains(&"HGHSM-FSLF31-MHSFFSBC"));
    }

    #[test]
    fn generated_static_tables_exactly_match_schedule_vars() {
        let parsed = parse_schedule_vars(SCHEDULE_VARS)
            .unwrap_or_else(|error| panic!("cannot parse schedule.vars: {error}"));

        assert_eq!(PREDEFINED_STRATEGIES.len(), parsed.strategies.len());
        for (actual, expected) in PREDEFINED_STRATEGIES.iter().zip(&parsed.strategies) {
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.definition, expected.definition);
        }

        assert_eq!(GENERATED_SCHEDULES.len(), parsed.schedules.len());
        for (actual, expected) in GENERATED_SCHEDULES.iter().zip(&parsed.schedules) {
            assert_static_schedule_matches(actual.name, actual.cells, expected);
        }
        assert_schedule_map_matches(
            PREPROCESSING_MAP,
            GENERATED_PREPROCESSING_SCHEDULE_NAMES,
            &parsed.preprocessing_map,
        );
        assert_schedule_map_matches(
            SEARCH_MAP,
            GENERATED_SEARCH_SCHEDULE_NAMES,
            &parsed.search_map,
        );
    }

    #[test]
    fn predefined_strategy_name_print_matches_c_shape() {
        let printed =
            strategies_print_predefined_string(true).unwrap_or_else(|error| panic!("{error}"));

        assert!(printed.starts_with(FIRST_STRATEGY));
        assert!(printed.ends_with('\n'));
        assert!(printed.lines().count() > 400);
        assert!(!printed.contains(" = "));
    }

    #[test]
    fn predefined_strategy_full_print_includes_definition() {
        let printed =
            strategies_print_predefined_string(false).unwrap_or_else(|error| panic!("{error}"));

        assert!(printed.starts_with(&format!("{FIRST_STRATEGY} = \n#{FIRST_STRATEGY}\n")));
        assert!(printed.contains("selection_strategy: PSelectComplexExceptUniqMaxHorn"));
    }

    #[test]
    fn get_heuristic_with_name_parses_predefined_strategy() {
        let mut params = HeuristicParmsCell::default();

        get_heuristic_with_name(FIRST_STRATEGY, &mut params)
            .unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(params.heuristic_name, "Default");
        assert_eq!(params.selection_strategy, "PSelectComplexExceptUniqMaxHorn");
        assert_eq!(params.forward_demod, RewriteLevel::FullRewrite);
    }

    #[test]
    fn get_heuristic_with_name_rejects_unknown_strategy() {
        let mut params = HeuristicParmsCell::default();
        let error = get_heuristic_with_name("Missing", &mut params).unwrap_err();

        assert_eq!(error.code(), ErrorCode::OTHER_ERROR);
        assert_eq!(
            error.message(),
            "Error: Configuration name Missing not found."
        );
    }

    #[test]
    fn schedule_string_distance_matches_c_positional_difference() {
        assert_eq!(schedule_string_distance("ABC", "ABC"), 0);
        assert_eq!(schedule_string_distance("ABC", "AXC"), 1);
        assert_eq!(schedule_string_distance("ABC", "AXCD"), 2);
        assert_eq!(schedule_string_distance("ABCD", "AX"), 3);
    }

    #[test]
    fn class_selection_uses_c_exact_and_largest_same_distance_tie_breaks() {
        const EMPTY_SCHEDULE: &[StaticScheduleCell] = &[];
        let classes = vec![
            ScheduleClass {
                key: "AAAA",
                schedule: EMPTY_SCHEDULE,
                class_size: 1,
            },
            ScheduleClass {
                key: "AAAB",
                schedule: EMPTY_SCHEDULE,
                class_size: 5,
            },
            ScheduleClass {
                key: "AAAC",
                schedule: EMPTY_SCHEDULE,
                class_size: 0,
            },
        ];

        let (partial, distance) =
            select_schedule_class("AAAD", &classes).expect("non-empty schedule map");
        assert_eq!(partial.key, "AAAB");
        assert_eq!(distance, 1);

        let (exact, distance) =
            select_schedule_class("AAAC", &classes).expect("non-empty schedule map");
        assert_eq!(exact.key, "AAAC");
        assert_eq!(distance, 0);
    }

    #[test]
    fn generated_schedule_tables_resolve_preprocessing_search_and_default() {
        let default_schedule = get_default_schedule().unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(default_schedule.len(), 8);
        assert_eq!(
            default_schedule
                .first()
                .map(|cell| cell.heuristic_name.as_str()),
            Some("G-E--_208_C18C--_F1_SE_CS_SP_PS_S5PRR_RG_S04AN")
        );
        assert_eq!(
            default_schedule.first().map(|cell| cell.ordering),
            Some(TermOrdering::NoOrdering)
        );

        let preprocessing =
            get_preprocessing_schedule("FSLMSMSLSSSNFFN").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(preprocessing.matched_class, "FSLMSMSLSSSNFFN");
        assert_eq!(preprocessing.distance, 0);
        assert_eq!(preprocessing.class_size, 456);
        assert_eq!(preprocessing.schedule.len(), 4);
        assert_eq!(
            preprocessing.schedule[0].heuristic_name,
            "G-E--_008_C45_F1_PI_SE_Q4_CS_SP_S4SI"
        );

        let search =
            get_search_schedule("FGHSF-FSLM21-MFFFFFNN").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(search.matched_class, "FGHSF-FSLM21-MFFFFFNN");
        assert_eq!(search.distance, 0);
        assert_eq!(search.class_size, 523);
        assert_eq!(search.schedule.len(), 11);
        assert_eq!(
            search.schedule[10].heuristic_name, "<placeholder>",
            "search schedules preserve the C placeholder cell for later insertion"
        );
    }

    #[test]
    fn generated_schedule_partial_match_reports_selected_class() {
        let resolved =
            get_search_schedule("FGHSF-FSLM21-MFFFFFNX").unwrap_or_else(|error| panic!("{error}"));

        assert_eq!(resolved.matched_class, "FGHSF-FSLM21-MFFFFFNN");
        assert_eq!(resolved.distance, 1);
        assert_eq!(resolved.class_size, 523);
    }

    #[test]
    fn schedule_times_init_preserves_single_core_c_limits() {
        let mut schedule = sample_schedule(&[0.25, 0.25, 0.5]);
        schedule_times_init(&mut schedule, 0.0, None);

        assert_eq!(schedule[0].time_absolute, DEFAULT_SCHED_TIME_LIMIT / 4);
        assert_eq!(schedule[1].time_absolute, DEFAULT_SCHED_TIME_LIMIT / 4);
        assert_eq!(schedule[2].time_absolute, u64::MAX);

        schedule_times_init(&mut schedule, 10.0, Some(100));
        assert_eq!(schedule[0].time_absolute, 22);
        assert_eq!(schedule[1].time_absolute, 22);
        assert_eq!(schedule[2].time_absolute, 46);
    }

    #[test]
    fn schedule_times_init_multi_core_handles_preprocessing_and_search_shapes() {
        let mut preprocessing = sample_schedule(&[0.5, 0.25, 0.25]);
        let mut cores = 2;
        let report =
            schedule_times_init_multi_core(&mut preprocessing, 0.0, 100.0, true, &mut cores, false);

        assert_eq!(report.scheduled, 2);
        assert_eq!(report.cores, 2);
        assert_eq!(report.limit, 100);
        assert_eq!(report.total_time, 200);
        assert_eq!(preprocessing.len(), 2);
        assert_eq!(preprocessing[0].cores, 1);
        assert_eq!(preprocessing[1].cores, 1);
        assert_eq!(preprocessing[0].time_absolute, 100);
        assert_eq!(preprocessing[1].time_absolute, 100);

        let mut search = sample_schedule(&[0.6, 0.6, 0.1]);
        let mut search_cores = 1;
        let search_report = schedule_times_init_multi_core(
            &mut search,
            0.0,
            100.0,
            false,
            &mut search_cores,
            false,
        );
        assert_eq!(search_report.scheduled, 2);
        assert_eq!(search_report.total_time, 100);
        assert_eq!(search[0].time_absolute, 60);
        assert_eq!(search[1].time_absolute, 40);
    }

    #[test]
    fn placeholder_schedule_insertion_preserves_c_mutation_shape() {
        let preprocessing = vec![schedule_cell("preproc", 0.7)];
        let mut search = vec![
            schedule_cell("first", 0.6),
            schedule_cell("second", 0.3),
            schedule_cell("<placeholder>", 0.0),
        ];

        initialize_placeholder_search_schedule(&mut search, &preprocessing, false)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(search.len(), 2);
        assert_eq!(search[0].heuristic_name, "first");

        let mut forced = vec![
            schedule_cell("first", 0.6),
            schedule_cell("second", 0.3),
            schedule_cell("<placeholder>", 0.0),
        ];
        initialize_placeholder_search_schedule(&mut forced, &preprocessing, true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(forced[0].heuristic_name, "first");
        assert_eq!(forced[1].heuristic_name, "preproc");
        assert_eq!(forced[2].heuristic_name, "second");
        assert_float_eq(forced[0].time_fraction, 0.54);
        assert_float_eq(forced[1].time_fraction, 0.1);

        let mut already_present = vec![
            schedule_cell("preproc", 0.6),
            schedule_cell("<placeholder>", 0.0),
            schedule_cell("after", 0.1),
        ];
        initialize_placeholder_search_schedule(&mut already_present, &preprocessing, true)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(already_present.len(), 1);
        assert_eq!(already_present[0].heuristic_name, "preproc");
    }

    #[test]
    fn filtered_default_schedule_keeps_c_last_fraction_quirk() {
        let default = vec![
            schedule_cell("keep-a", 0.1),
            schedule_cell("drop", 0.1),
            schedule_cell("keep-b", 0.1),
        ];
        let exhausted = vec![schedule_cell("drop", 0.1)];

        let filtered = get_filtered_default_schedule(&default, &exhausted);

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].heuristic_name, "keep-a");
        assert_eq!(filtered[1].heuristic_name, "keep-b");
        assert_float_eq(filtered[0].time_fraction, 0.5);
        assert_float_eq_msg(
            filtered[1].time_fraction,
            0.1,
            "C updates entries before last_filtered, leaving the final kept cell unchanged",
        );
    }

    fn sample_schedule(fractions: &[f64]) -> Vec<ScheduleCell> {
        fractions
            .iter()
            .enumerate()
            .map(|(index, fraction)| schedule_cell(&format!("s{index}"), *fraction))
            .collect()
    }

    fn schedule_cell(name: &str, time_fraction: f64) -> ScheduleCell {
        ScheduleCell {
            heuristic_name: name.to_owned(),
            ordering: TermOrdering::NoOrdering,
            sine: None,
            time_fraction,
            time_absolute: 1,
            cores: 1,
        }
    }

    fn assert_float_eq(actual: f64, expected: f64) {
        assert_float_eq_msg(actual, expected, "unexpected floating-point value");
    }

    fn assert_float_eq_msg(actual: f64, expected: f64, message: &str) {
        assert!((actual - expected).abs() < f64::EPSILON, "{message}");
    }

    fn assert_static_schedule_matches(
        actual_name: &str,
        actual_cells: &[StaticScheduleCell],
        expected: &ParsedSchedule,
    ) {
        assert_eq!(actual_name, expected.name);
        assert_eq!(actual_cells.len(), expected.cells.len());
        for (actual, expected) in actual_cells.iter().zip(&expected.cells) {
            assert_eq!(actual.heuristic_name, expected.heuristic_name);
            assert_eq!(actual.ordering.name(), expected.ordering);
            assert_eq!(actual.sine, expected.sine.as_deref());
            assert_eq!(
                actual.time_fraction.to_bits(),
                expected.time_fraction.to_bits()
            );
            assert_eq!(actual.time_absolute, expected.time_absolute);
            assert_eq!(actual.cores, expected.cores);
        }
    }

    fn assert_schedule_map_matches(
        actual: &[ScheduleClass],
        actual_schedule_names: &[&str],
        expected: &[ParsedScheduleClass],
    ) {
        assert_eq!(actual.len(), expected.len());
        assert_eq!(actual_schedule_names.len(), expected.len());
        for ((actual, actual_schedule_name), expected) in
            actual.iter().zip(actual_schedule_names).zip(expected)
        {
            assert_eq!(actual.key, expected.key);
            assert_eq!(*actual_schedule_name, expected.schedule_name);
            assert_eq!(actual.class_size, expected.class_size);
        }
    }
}
