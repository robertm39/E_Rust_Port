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

### Change Later

- `ScheduleTimesInit` casts `time_fraction * limit` directly to `rlim_t`, truncating fractional seconds for all but the final strategy; `ScheduleTimesInitMultiCore` instead uses `ceil` and may allocate preprocessing schedules more total CPU seconds than wall-clock seconds because it multiplies by per-strategy cores. Rust preserves both shapes in pure helpers over owned schedule copies.
- `InitializePlaceholderSearchSchedule` mutates the generated search schedule in place: without forced preprocessing it writes a NULL terminator at the placeholder, and with forced preprocessing it overwrites the placeholder with the selected preprocessing strategy, rescales earlier fractions, then swaps the inserted entry into slot 1. Rust models this over owned vectors so callers can avoid mutating shared generated data; if C-style cross-run mutation becomes observable, scheduler ownership will need another audit.
- `GetFilteredDefaultSchedule` also mutates the generated default schedule in place while filtering out strategies already run by an exhausted schedule. Its fraction-reset loop updates entries before `last_filtered` and leaves the final kept strategy's old fraction untouched. Rust preserves that quirk over a returned owned copy.
- `ExecuteScheduleMultiCore` child processes set both `h_parms->heuristic_name` and `h_parms->order_params.ordertype` from the selected `ScheduleCell` before returning to the caller, which then reparses the named heuristic into the same parameter cell. Rust's executable worker bridge currently re-execs selected named strategies through the normal `--select-strategy` path, so exact schedule-cell `ordertype` preload remains a compatibility item if generated schedule cells ever rely on an ordering not already encoded in the named strategy.
- `ExecuteScheduleMultiCore` is a process controller, not just a schedule iterator: it installs a SIGTERM scheduler handler that counts the signal and restores the default handler, forks children through `EGPCtrlCreate`, redirects selected strategy output, returns the child schedule index only in the child process, prints the winning child output in the parent, and exits the parent with the child status. Rust uses an explicit current-binary worker-process bridge for supported executable `--auto-schedule`/`--satauto-schedule` preprocessing schedules and their nested search schedules. It now preserves parent-side schedule summaries, child-output capture, winning-output replay, `PARENT_REQUEST` status on a caught scheduler SIGTERM, filtered default-schedule retry, parent-versus-descendant CPU accounting, and the nested child/parent resource-footer order. Standard input is snapshotted once and replayed in each worker so exec does not turn `-` into EOF. Safe cross-platform exec cannot inherit C's already-parsed/preprocessed heap graph, so stable file inputs are deliberately reparsed and reprocessed in workers; the compatibility and startup-cost decision is recorded in [`experiments/2026-07-16-034-multicore-fork-compatibility/FINDINGS.md`](../../../experiments/2026-07-16-034-multicore-fork-compatibility/FINDINGS.md).
- The C default-schedule retry computes `remaining_time` from total CPU including children, then calls `ExecuteScheduleMultiCore`, whose initialization subtracts current parent process CPU time from that remaining value. Rust preserves both clocks separately: descendant-inclusive CPU computes the retry budget and self CPU initializes the retry schedule. On Windows, waited-child process times plus the child's reported aggregate footer propagate nested worker usage into the outer parent; Unix continues to use `getrusage(RUSAGE_SELF/RUSAGE_CHILDREN)`.
- `ExecuteScheduleMultiCore` has no guard for a schedule cell whose `cores` requirement exceeds `max_cores` when no subprocess is active. The inner spawn loop will not start the cell, `EGPCtrlSetGetResult` has no active process to observe, and the outer loop can spin forever. Rust's reusable coordinator reports this as an explicit scheduling diagnostic; if byte-for-byte compatibility ever requires the hang, isolate it behind a low-level compatibility mode.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
