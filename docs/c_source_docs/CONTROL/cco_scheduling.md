<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTROL / cco_scheduling

## Source Files

- [CONTROL/cco_scheduling.h](../../../eprover/CONTROL/cco_scheduling.h)
- [CONTROL/cco_scheduling.c](../../../eprover/CONTROL/cco_scheduling.c)

## Purpose

Some simple data types and code to implement quick-and-dirty strategy scheduling for E. <1> Wed May 22 22:33:40 CEST 2013 New

Within the source tree, this unit belongs to `CONTROL`. High-level proof-control layer: preprocessing, saturation loop orchestration, inference scheduling, contraction, SInE, splitting, server/session management, and higher-order inference control.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ScheduleCell`
- `Schedule_p`

### Macros And Constants

- `CCO_SCHEDULING`
- `DEFAULT_SCHED_TIME_LIMIT`
- `RETRY_DEFAULT_SCHEDULE_THRESHOLD`
- `SCHEDULE_DONE`

### Globals

- None found in the source scan.

### Exported Functions

- `Schedule_p GetFilteredDefaultSchedule(Schedule_p exhausted_sched)`
- `int ExecuteScheduleMultiCore(ScheduleCell strats[], HeuristicParms_p h_parms, bool print_rusage, int wc_time_limit, int compute_cores_per_schedule, int max_cores, bool serialize)`
- `void InitializePlaceholderSearchSchedule(Schedule_p search_sched, Schedule_p preproc_sched, bool force_preproc)`
- `void ScheduleTimesInit(ScheduleCell sched[], double time_used)`
- `void ScheduleTimesInitMultiCore(ScheduleCell sched[], double time_used, double time_limit, bool preprocessing_schedule, int* cores, bool serialize)`

## Implementation Notes

### Internal Functions

- `name_in_schedule`

### Source-Level Behavior

- `name_in_schedule`: Is the heuristic name in the schedule?
- `ScheduleTimesInitMultiCore`: If preprocessing_schedule is true (used for scheduling preprocessing) based on the time fraction the number of cores allocated to the preprocessor will be computed and stored in cores. Cores must be initialized to the prefered maximal number of cores and if this number is smaller than the number of preprocessors, then it is going to be set to the number of...
- `ExecuteScheduleMultiCore`: Execute the hard-coded strategy schedule.
- `InitializePlaceholderSearchSchedule`: Find the placeholder position in search sched and replace it with NULL (terminate the schedule array) if we do not need to insert preprocessing schedule into search schedule, otherwise replace it with the name of preprocessing schedule.
- `GetDefaultSchedule`: After a schedule is exhausted, then turn back to the default schedule and filter the configurations that have not be run from it.

### Dependencies

- `"cco_scheduling.h"`
- `<cco_gproc_ctrl.h>`
- `<che_hcb.h>`
- `<che_new_autoschedule.h>`
- `<cio_signals.h>`
- `<sys/mman.h>`
- `<sys/types.h>`
- `<sys/wait.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `CCO_SCHEDULING`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTROL/cco_scheduling.h`, `CONTROL/cco_scheduling.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CONTROL` covering 2 source file(s), about 502 lines, 7 scanned public declarations, 1 scanned internal function definitions, and 5 structured function-comment blocks.
- Strategy scheduling; preserve time/core split behavior and schedule serialization compatibility.
- Proof-control code. These units connect preprocessing, inference generation, contraction, scheduling, and proof output, so behavior is often defined by call ordering.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change-Later Observations

- `InitializePlaceholderSearchSchedule` mutates the generated search schedule in place: without forced preprocessing it writes a NULL terminator at the placeholder, and with forced preprocessing it overwrites the placeholder with the selected preprocessing strategy, rescales earlier fractions, then swaps the inserted entry into slot 1. Rust schedule parsing preserves the placeholder as a normal cell for now; a later executable scheduler should decide whether to clone before mutation or model the C global-array mutation explicitly.
- `GetFilteredDefaultSchedule` also mutates the generated default schedule in place while filtering out strategies already run by an exhausted schedule. This relies on generated static arrays being writable process state. A Rust scheduler should prefer owned per-run schedule copies unless reference tests show cross-run mutation is observable.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
