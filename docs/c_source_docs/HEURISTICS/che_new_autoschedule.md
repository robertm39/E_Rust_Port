<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_new_autoschedule

## Source Files

- [HEURISTICS/che_new_autoschedule.h](../../../eprover/HEURISTICS/che_new_autoschedule.h)
- [HEURISTICS/che_new_autoschedule.c](../../../eprover/HEURISTICS/che_new_autoschedule.c)

## Purpose

Code for new (symbolic) autoschedule.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Petar Vukmirovic, Stephan Schulz, Stephan Schulz (schulz@eprover.org), Petar Vukmirovic

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `StrSchedPair`
- `StrStrPair`

### Macros And Constants

- `CHE_NEW_AUTOSCHEDULE`
- `DEFAULT_MASK`
- `RAW_DEFAULT_MASK`

### Globals

- None found in the source scan.

### Exported Functions

- `ScheduleCell* GetDefaultSchedule()`
- `ScheduleCell* GetPreprocessingSchedule(const char* problem_category)`
- `ScheduleCell* GetSearchSchedule(const char* problem_category)`
- `void GetHeuristicWithName(const char* name, HeuristicParms_p target)`
- `void StrategiesPrintPredefined(FILE* out, bool name_only)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `class_to_schedule`: Given a class name, return a schedule. If there is an exact match for the class, use the associated schedule, otherwise use the schedule associated to the largest of the classes with minimal string distance.
- `StrategiesPrintPredefined`: Print all predefined strategies.
- `GetPreprocessingSchedule`: Get preprocessing schedule for a class.
- `GetSearchSchedule`: Get search schedule for a class.
- `GetHeuristicWithName`: Given a name, find and parse a heuristic into the provided cell.
- `GetDefaultSchedule`: Return the default (fallback) schedule.

### Dependencies

- `"che_hcb.h"`
- `"che_new_autoschedule.h"`
- `"schedule.vars"`
- `<cco_scheduling.h>`

### Compile-Time Conditions

- `CHE_NEW_AUTOSCHEDULE`
- `FILE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_new_autoschedule.h`, `HEURISTICS/che_new_autoschedule.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 264 lines, 7 scanned public declarations, 0 scanned internal function definitions, and 6 structured function-comment blocks.
- Built-in automatic schedule definitions; treat generated strategy constants as compatibility data.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `schedule.vars` is generated C data included directly into `che_new_autoschedule.c`; `StrategiesPrintPredefined` and `GetHeuristicWithName` scan the `conf_map` array linearly and treat its strings as authoritative compatibility data, while `GetPreprocessingSchedule`/`GetSearchSchedule` return pointers to generated `ScheduleCell` arrays selected by class-map lookup. Rust now includes and parses the generated `conf_map`, `ScheduleCell`, `preproc_sched_map`, `search_sched_map`, and `_DEFAULT_SCHEDULE` data for lookup. A build-time extractor or checked-in Rust table may be cleaner later, but only after reference tests pin exact update and formatting behavior.
- Plain executable `--auto` uses these generated tables as configuration names rather than as a process schedule: it selects the first preprocessing/search cell for the classified problem and then parses the named `conf_map` heuristic. Scheduled executable modes additionally treat the `ScheduleCell.ordering` field as child handoff state before the named heuristic is reparsed. Rust now uses the parsed generated tables for the supported first-order paths and preserves that ordering preload in the temporary in-process scheduler bridge; the pointer-returning/mutable-array behavior remains confined to scheduler compatibility helpers.
- `class_to_schedule` calls `StrDistance`, which is only positional character mismatch plus length difference, not edit distance. Ties use the largest generated `class_size`, exact matches stop immediately, and partial matches print the chosen class through `GlobalOut`. Rust exposes the selected class and distance so the eventual executable scheduler can reproduce that comment instead of hiding it inside lookup.
- `GetHeuristicWithName` reparses the selected strategy text into an existing `HeuristicParmsCell`, relying on the ordered sparse `HeuristicParmsParseInto` behavior to leave omitted fields untouched. Preserve that mutation style for compatibility; a future typed strategy format should make partial overrides explicit if the generated strategy table is ever normalized.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
