use crate::basics::error::{Diagnostic, ErrorCode};
use crate::control::gproc_ctrl::{EGPCtrl, EGPCtrlSet};
use crate::heuristics::new_autoschedule::{
    get_filtered_default_schedule, schedule_times_init_multi_core, ScheduleCell,
    ScheduleMultiCoreInitReport, RETRY_DEFAULT_SCHEDULE_THRESHOLD, SCHEDULE_DONE,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

const PROCESS_POLL_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleExecutionOutcome {
    Result {
        index: usize,
        name: String,
        exit_status: i32,
    },
    Exhausted,
}

impl ScheduleExecutionOutcome {
    #[must_use]
    pub const fn c_return_value(&self) -> i32 {
        match self {
            Self::Result { exit_status, .. } => *exit_status,
            Self::Exhausted => SCHEDULE_DONE,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleExecutionReport {
    pub init_report: ScheduleMultiCoreInitReport,
    pub outcome: ScheduleExecutionOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleExecutionWithRetryReport {
    pub primary: ScheduleExecutionReport,
    pub retry: Option<ScheduleExecutionReport>,
}

impl ScheduleExecutionWithRetryReport {
    #[must_use]
    pub fn outcome(&self) -> &ScheduleExecutionOutcome {
        self.retry
            .as_ref()
            .map_or(&self.primary.outcome, |report| &report.outcome)
    }

    #[must_use]
    pub const fn c_return_value(&self) -> i32 {
        match &self.retry {
            Some(report) => report.outcome.c_return_value(),
            None => self.primary.outcome.c_return_value(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScheduleExecutionConfig {
    pub time_used: f64,
    pub wc_time_limit: f64,
    pub preprocessing_schedule: bool,
    pub max_cores: i32,
    pub serialize: bool,
}

pub fn execute_schedule_multi_core_with_default_retry<F, T>(
    schedule: &mut Vec<ScheduleCell>,
    default_schedule: &[ScheduleCell],
    config: ScheduleExecutionConfig,
    output: &mut impl Write,
    mut total_time_used: T,
    mut spawn: F,
) -> Result<ScheduleExecutionWithRetryReport, Diagnostic>
where
    F: FnMut(usize, &ScheduleCell, &mut dyn Write) -> Result<EGPCtrl, Diagnostic>,
    T: FnMut() -> f64,
{
    let primary = execute_schedule_multi_core(schedule, config, output, &mut spawn)?;
    if config.preprocessing_schedule
        || !matches!(primary.outcome, ScheduleExecutionOutcome::Exhausted)
    {
        return Ok(ScheduleExecutionWithRetryReport {
            primary,
            retry: None,
        });
    }

    let remaining_time = config.wc_time_limit - total_time_used();
    if remaining_time <= RETRY_DEFAULT_SCHEDULE_THRESHOLD {
        return Ok(ScheduleExecutionWithRetryReport {
            primary,
            retry: None,
        });
    }

    let mut retry_schedule = get_filtered_default_schedule(default_schedule, schedule);
    writeln!(
        output,
        "% executing default schedule for {remaining_time} seconds."
    )
    .map_err(|error| output_error(&error))?;
    let retry = execute_schedule_multi_core(
        &mut retry_schedule,
        ScheduleExecutionConfig {
            time_used: 0.0,
            wc_time_limit: remaining_time,
            preprocessing_schedule: false,
            max_cores: config.max_cores,
            serialize: config.serialize,
        },
        output,
        &mut spawn,
    )?;

    Ok(ScheduleExecutionWithRetryReport {
        primary,
        retry: Some(retry),
    })
}

pub fn execute_schedule_multi_core<F>(
    schedule: &mut Vec<ScheduleCell>,
    config: ScheduleExecutionConfig,
    output: &mut impl Write,
    mut spawn: F,
) -> Result<ScheduleExecutionReport, Diagnostic>
where
    F: FnMut(usize, &ScheduleCell, &mut dyn Write) -> Result<EGPCtrl, Diagnostic>,
{
    let mut cores = config.max_cores.max(1);
    let init_report = schedule_times_init_multi_core(
        schedule,
        config.time_used,
        config.wc_time_limit,
        config.preprocessing_schedule,
        &mut cores,
        config.serialize,
    );
    write_schedule_report(output, &init_report)?;

    let mut controls = EGPCtrlSet::new();
    let mut descriptors = BTreeMap::new();
    let mut next = 0_usize;

    loop {
        let spawned = spawn_eligible_processes(
            schedule,
            cores,
            &mut next,
            &mut controls,
            &mut descriptors,
            output,
            &mut spawn,
        )?;

        if controls.is_empty() && next >= schedule.len() {
            writeln!(output, "% Schedule exhausted").map_err(|error| output_error(&error))?;
            return Ok(ScheduleExecutionReport {
                init_report,
                outcome: ScheduleExecutionOutcome::Exhausted,
            });
        }

        if controls.is_empty() && !spawned {
            let Some(cell) = schedule.get(next) else {
                continue;
            };
            return Err(scheduling_error(format!(
                "Cannot schedule {} requiring {} cores with {cores} total cores",
                cell.heuristic_name, cell.cores
            )));
        }

        if let Some(descriptor) =
            controls.get_result_from_pipes_timeout(PROCESS_POLL_TIMEOUT, output)?
        {
            let index = descriptors.get(&descriptor).copied().ok_or_else(|| {
                scheduling_error("Winning schedule process is missing its schedule index")
            })?;
            let control = controls.find_proc(descriptor).ok_or_else(|| {
                scheduling_error("Winning schedule process disappeared before result handling")
            })?;
            let name = control.name().unwrap_or("").to_owned();
            let exit_status = control.exit_status();
            writeln!(output, "% Result found by {name}").map_err(|error| output_error(&error))?;
            output
                .write_all(control.output().view_bytes())
                .map_err(|error| output_error(&error))?;
            controls.clear(true)?;
            return Ok(ScheduleExecutionReport {
                init_report,
                outcome: ScheduleExecutionOutcome::Result {
                    index,
                    name,
                    exit_status,
                },
            });
        }
    }
}

fn spawn_eligible_processes<F>(
    schedule: &[ScheduleCell],
    max_cores: i32,
    next: &mut usize,
    controls: &mut EGPCtrlSet,
    descriptors: &mut BTreeMap<crate::control::esession::Descriptor, usize>,
    output: &mut impl Write,
    spawn: &mut F,
) -> Result<bool, Diagnostic>
where
    F: FnMut(usize, &ScheduleCell, &mut dyn Write) -> Result<EGPCtrl, Diagnostic>,
{
    let mut spawned = false;
    let max_cores = usize_from_i32_saturating(max_cores);
    while let Some(cell) = schedule.get(*next) {
        let required_cores = schedule_cell_cores(cell);
        if max_cores.saturating_sub(controls.cores_reserved()) < required_cores {
            break;
        }
        let control = spawn(*next, cell, output)?;
        let descriptor = control.descriptor().ok_or_else(|| {
            scheduling_error(format!(
                "Spawned schedule process {} has no descriptor",
                cell.heuristic_name
            ))
        })?;
        let _previous = descriptors.insert(descriptor, *next);
        let _replaced = controls.add_proc(control)?;
        *next += 1;
        spawned = true;
    }
    Ok(spawned)
}

fn write_schedule_report(
    output: &mut impl Write,
    report: &ScheduleMultiCoreInitReport,
) -> Result<(), Diagnostic> {
    writeln!(
        output,
        "% Scheduled {} strats onto {} cores with {} seconds ({} total)",
        report.scheduled, report.cores, report.limit, report.total_time
    )
    .map_err(|error| output_error(&error))
}

fn schedule_cell_cores(cell: &ScheduleCell) -> usize {
    usize_from_i32_saturating(cell.cores).max(1)
}

fn usize_from_i32_saturating(value: i32) -> usize {
    usize::try_from(value).unwrap_or(0)
}

fn scheduling_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(ErrorCode::INTERFACE_ERROR, message)
}

fn output_error(error: &std::io::Error) -> Diagnostic {
    Diagnostic::new(
        ErrorCode::FILE_ERROR,
        format!("Could not write schedule execution output: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        execute_schedule_multi_core, execute_schedule_multi_core_with_default_retry,
        ScheduleExecutionConfig, ScheduleExecutionOutcome,
    };
    use crate::basics::error::ErrorCode;
    use crate::control::gproc_ctrl::EGPCtrl;
    use crate::heuristics::new_autoschedule::{ScheduleCell, SCHEDULE_DONE};
    use crate::heuristics::to_params::TermOrdering;
    use std::process::Command;

    #[test]
    fn execution_reports_first_success_and_prints_child_output() {
        let mut schedule = vec![
            schedule_cell("fail", 0.1, 1),
            schedule_cell("prove", 0.1, 1),
            schedule_cell("later", 0.1, 1),
        ];
        let mut output = Vec::new();

        let report = execute_schedule_multi_core(
            &mut schedule,
            config(10.0, 2),
            &mut output,
            |_, cell, output| {
                let status = if cell.heuristic_name == "prove" {
                    "% SZS status Theorem"
                } else {
                    "no result"
                };
                EGPCtrl::spawn_command_reporting(
                    status_command(status, 0),
                    cell.heuristic_name.clone(),
                    usize::try_from(cell.cores).unwrap_or(1),
                    cell.time_absolute,
                    output,
                )
            },
        )
        .unwrap();

        assert_eq!(
            report.outcome,
            ScheduleExecutionOutcome::Result {
                index: 1,
                name: "prove".to_owned(),
                exit_status: 0
            }
        );
        assert_eq!(report.outcome.c_return_value(), 0);
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("% Scheduled 3 strats onto 2 cores with 10 seconds"));
        assert!(printed.contains("% Starting fail"));
        assert!(printed.contains("% Starting prove"));
        assert!(printed.contains("% Result found by prove"));
        assert!(printed.contains("% SZS status Theorem"));
    }

    #[test]
    fn execution_respects_core_capacity_before_spawning_later_cells() {
        let mut schedule = vec![
            schedule_cell("wide-failure", 0.1, 2),
            schedule_cell("narrow-success", 0.1, 1),
        ];
        let mut output = Vec::new();
        let mut spawned_names = Vec::new();

        let report = execute_schedule_multi_core(
            &mut schedule,
            config(10.0, 2),
            &mut output,
            |_, cell, output| {
                spawned_names.push(cell.heuristic_name.clone());
                let status = if cell.heuristic_name == "narrow-success" {
                    "% SZS status Unsatisfiable"
                } else {
                    "no result"
                };
                EGPCtrl::spawn_command_reporting(
                    status_command(status, 0),
                    cell.heuristic_name.clone(),
                    usize::try_from(cell.cores).unwrap_or(1),
                    cell.time_absolute,
                    output,
                )
            },
        )
        .unwrap();

        assert_eq!(spawned_names, ["wide-failure", "narrow-success"]);
        assert_eq!(
            report.outcome,
            ScheduleExecutionOutcome::Result {
                index: 1,
                name: "narrow-success".to_owned(),
                exit_status: 0
            }
        );
    }

    #[test]
    fn execution_reports_schedule_exhaustion() {
        let mut schedule = vec![
            schedule_cell("fail-a", 0.1, 1),
            schedule_cell("fail-b", 0.1, 1),
        ];
        let mut output = Vec::new();

        let report = execute_schedule_multi_core(
            &mut schedule,
            config(10.0, 2),
            &mut output,
            |_, cell, output| {
                EGPCtrl::spawn_command_reporting(
                    status_command("no result", 0),
                    cell.heuristic_name.clone(),
                    usize::try_from(cell.cores).unwrap_or(1),
                    cell.time_absolute,
                    output,
                )
            },
        )
        .unwrap();

        assert_eq!(report.outcome, ScheduleExecutionOutcome::Exhausted);
        assert_eq!(report.outcome.c_return_value(), SCHEDULE_DONE);
        assert!(String::from_utf8(output)
            .unwrap()
            .contains("% Schedule exhausted\n"));
    }

    #[test]
    fn execution_retries_filtered_default_schedule_when_time_remains() {
        let mut schedule = vec![
            schedule_cell("fail-a", 0.1, 1),
            schedule_cell("fail-b", 0.1, 1),
        ];
        let default_schedule = vec![
            schedule_cell("fail-a", 0.1, 1),
            schedule_cell("default-prove", 0.1, 1),
            schedule_cell("default-later", 0.1, 1),
        ];
        let mut output = Vec::new();
        let mut spawned_names = Vec::new();

        let report = execute_schedule_multi_core_with_default_retry(
            &mut schedule,
            &default_schedule,
            config(10.0, 2),
            &mut output,
            || 4.0,
            |_, cell, output| {
                spawned_names.push(cell.heuristic_name.clone());
                let status = if cell.heuristic_name == "default-prove" {
                    "% SZS status Theorem"
                } else {
                    "no result"
                };
                EGPCtrl::spawn_command_reporting(
                    status_command(status, 0),
                    cell.heuristic_name.clone(),
                    usize::try_from(cell.cores).unwrap_or(1),
                    cell.time_absolute,
                    output,
                )
            },
        )
        .unwrap();

        assert_eq!(report.primary.outcome, ScheduleExecutionOutcome::Exhausted);
        assert_eq!(
            report.outcome(),
            &ScheduleExecutionOutcome::Result {
                index: 0,
                name: "default-prove".to_owned(),
                exit_status: 0,
            }
        );
        assert_eq!(report.c_return_value(), 0);
        assert_eq!(
            spawned_names,
            ["fail-a", "fail-b", "default-prove", "default-later"]
        );
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("% Schedule exhausted\n"));
        assert!(printed.contains("% executing default schedule for 6 seconds.\n"));
        assert!(printed.contains("% Result found by default-prove"));
    }

    #[test]
    fn execution_skips_default_retry_when_remaining_time_is_too_small() {
        let mut schedule = vec![schedule_cell("fail-a", 0.1, 1)];
        let default_schedule = vec![schedule_cell("default-prove", 0.1, 1)];
        let mut output = Vec::new();
        let mut spawned_names = Vec::new();

        let report = execute_schedule_multi_core_with_default_retry(
            &mut schedule,
            &default_schedule,
            config(10.0, 1),
            &mut output,
            || 9.0,
            |_, cell, output| {
                spawned_names.push(cell.heuristic_name.clone());
                EGPCtrl::spawn_command_reporting(
                    status_command("no result", 0),
                    cell.heuristic_name.clone(),
                    usize::try_from(cell.cores).unwrap_or(1),
                    cell.time_absolute,
                    output,
                )
            },
        )
        .unwrap();

        assert_eq!(report.primary.outcome, ScheduleExecutionOutcome::Exhausted);
        assert_eq!(report.retry, None);
        assert_eq!(report.c_return_value(), SCHEDULE_DONE);
        assert_eq!(spawned_names, ["fail-a"]);
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("% Schedule exhausted\n"));
        assert!(!printed.contains("executing default schedule"));
    }

    #[test]
    fn execution_rejects_unschedulable_cells_instead_of_spinning() {
        let mut schedule = vec![schedule_cell("too-wide", 1.0, 3)];
        let mut output = Vec::new();

        let error = execute_schedule_multi_core(
            &mut schedule,
            config(10.0, 2),
            &mut output,
            |_, cell, output| {
                EGPCtrl::spawn_command_reporting(
                    status_command("unused", 0),
                    cell.heuristic_name.clone(),
                    usize::try_from(cell.cores).unwrap_or(1),
                    cell.time_absolute,
                    output,
                )
            },
        )
        .unwrap_err();

        assert_eq!(error.code(), ErrorCode::INTERFACE_ERROR);
        assert!(error.message().contains("Cannot schedule too-wide"));
    }

    fn schedule_cell(name: &str, fraction: f64, cores: i32) -> ScheduleCell {
        ScheduleCell {
            heuristic_name: name.to_owned(),
            ordering: TermOrdering::NoOrdering,
            sine: None,
            time_fraction: fraction,
            time_absolute: 0,
            cores,
        }
    }

    fn config(wc_time_limit: f64, max_cores: i32) -> ScheduleExecutionConfig {
        ScheduleExecutionConfig {
            time_used: 0.0,
            wc_time_limit,
            preprocessing_schedule: false,
            max_cores,
            serialize: false,
        }
    }

    #[cfg(windows)]
    fn status_command(status: &str, exit_code: i32) -> Command {
        let mut command = Command::new("cmd");
        command.args(["/C", &format!("echo {status}& exit /B {exit_code}")]);
        command
    }

    #[cfg(unix)]
    fn status_command(status: &str, exit_code: i32) -> Command {
        let mut command = Command::new("sh");
        command.args([
            "-c",
            &format!("printf '%s\\n' '{status}'; exit {exit_code}"),
        ]);
        command
    }
}
