<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# INOUT / cio_commandline

## Source Files

- [INOUT/cio_commandline.h](../../../eprover/INOUT/cio_commandline.h)
- [INOUT/cio_commandline.c](../../../eprover/INOUT/cio_commandline.c)

## Purpose

Definitions for handling options and recognising non-option arguments. "Why don't you use getopt()?" - Implementations of getopt() seem to differ significantly between UNIX implementations. Finding out what the differences are and coding around them seems to be more work than writing this version

Within the source tree, this unit belongs to `INOUT`. Input/output substrate: scanners, parsers, command-line handling, streams, files, temp files, signals, network helpers, and output formatting.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CLStateCell`
- `CLState_p`
- `OptArgType`
- `OptCell`
- `Opt_p`

### Macros And Constants

- `CIO_COMMANDLINE`
- `CLStateCellAlloc()`
- `CLStateCellFree(junk)`
- `FORMAT_WIDTH`

### Globals

- None found in the source scan.

### Exported Functions

- `CLState_p CLStateAlloc(int argc, char* argv[])`
- `Opt_p CLStateGetOpt(CLState_p state, char** arg, OptCell options[])`
- `bool CLStateGetBoolArg(Opt_p option, char* arg)`
- `double CLStateGetFloatArg(Opt_p option, char* arg)`
- `int CLStateInsertArg(CLState_p state, char* arg)`
- `long CLStateGetIntArg(Opt_p option, char* arg)`
- `long CLStateGetIntArgCheckRange(Opt_p option, char* arg, long lower, long upper)`
- `void CLStateFree(CLState_p junk)`
- `void PrintOption(FILE* out, Opt_p option)`
- `void PrintOptions(FILE* out, OptCell option[], char* header)`

## Implementation Notes

### Internal Functions

- `append_option_desc`
- `find_long_opt`
- `find_short_opt`
- `print_start_of_str`
- `process_long_option`
- `process_short_option`
- `shift_array_left`

### Source-Level Behavior

- `print_start_of_str`: Print str up to the last blank character before the len's character or the first newline, whichever is first, followed by a newline. If there is no blank, break at character number len. Returns a pointer to the first character following the break, or NULL if the string was printed completely.
- `shift_array_left`: Shift a 0-terminated array of char* elements left by one, dropping the first element. Return false if no element is present.
- `find_long_opt`: Find an option entry by long name. Return NULL if not found.
- `find_short_opt`: Find an option entry by short name. Return NULL if not found.
- `process_long_option`: Process the long option that is found in state->argc[state->argi]: Find the option, check for argument, set *arg to an argument, update state.
- `process_short_option`: Process the short option that is found in state->argc[state->argi][state->sc_opt_c]: Find the option, check for argument, set *arg to an argument, update state.
- `append_option_desc`: Append a description of the given option to the DStr.
- `CLStateAlloc`: Allocate initialized Structure for the description of a (partially processed) command line.
- `CLStateFree`: Free a CLStateCell.
- `CLStateInsertArg`: Insert an additional argument at the end of state->argv, realloc for more memory if necessary. Return new state->argc value. arg is expected to be const, it is not copied!
- `CLStateGetOpt`: Return a pointer to the next unprocessed option, set arg to point to the argument (if present) or the default (if present).
- `CLStateGetFloatArg`: Return the numerical value of the argument if it is a well-formed (double) float, print an error message otherwise.
- `CLStateGetIntArg`: Return the numerical value of the argument if it is a well-formed long, print an error message otherwise.
- `CLStateGetIntArgCheckRange`: Return the numerical value of the argument if it is a well-formed long in the proper range, print an error message otherwise.
- `CLStateGetBoolArg`: Return the boolean value of the argument if it is either 'true' or 'false' long, print an error message otherwise.
- `PrintOption`: Print the formatted description of an option (generated from the information in an OptCell() to the desired stream.
- `PrintOptions`: Print the whole option array (terminated by an OptCell with type NoOption.

### Dependencies

- `"cio_commandline.h"`
- `<clb_dstrings.h>`

### Compile-Time Conditions

- `CIO_COMMANDLINE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `INOUT/cio_commandline.h`, `INOUT/cio_commandline.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `INOUT` covering 2 source file(s), about 928 lines, 15 scanned public declarations, 7 scanned internal function definitions, and 17 structured function-comment blocks.
- Command-line parser. Option compatibility for E executables depends on exact flag arity and default handling.
- Parsing and output code. Scanner state, token consumption, include handling, and fatal parse errors are part of the observable interface.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
