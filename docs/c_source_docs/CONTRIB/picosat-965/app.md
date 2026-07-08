<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTRIB/picosat-965 / app

## Source Files

- [CONTRIB/picosat-965/app.c](../../../../eprover/CONTRIB/picosat-965/app.c)

## Purpose

Support code for the vendored PicoSAT command-line application.

Within the source tree, this unit belongs to `CONTRIB/picosat-965`. Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `BUNZIP2`
- `GUNZIP`
- `GZIP`
- `USAGE`

### Globals

- None found in the source scan.

### Exported Functions

- `FILE * popen (const char *, const char*)`
- `extern void picosat_enter (PicoSAT *)`
- `extern void picosat_leave (PicoSAT *)`
- `int pclose (FILE *)`
- `static void (*sig_abrt_handler)`
- `static void (*sig_alarm_handler)`
- `static void (*sig_int_handler)`
- `static void (*sig_kill_handler)`
- `static void (*sig_segv_handler)`
- `static void (*sig_term_handler)`
- `static void (*sig_xcpu_handler)`
- `static void (*sig_xfsz_handler)`

## Implementation Notes

### Internal Functions

- `alarm_triggered`
- `bflush`
- `blocksol`
- `catch`
- `has_suffix`
- `interrupt_call_back`
- `message`
- `next`
- `next_assumption`
- `parse`
- `printa`
- `printi`
- `resetalarm`
- `resetsighandlers`
- `setalarm`
- `setsighandlers`
- `write_core_variables`
- `write_failed_assumptions`
- `write_to_file`

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"picosat.h"`
- `<assert.h>`
- `<ctype.h>`
- `<signal.h>`
- `<stdio.h>`
- `<stdlib.h>`
- `<string.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `NALLSIGNALS`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTRIB/picosat-965/app.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `CONTRIB/picosat-965` covering 1 source file(s), about 1200 lines, 12 scanned public declarations, 19 scanned internal function definitions, and 0 structured function-comment blocks.
- PicoSAT app support code used by the vendored solver utilities; keep it separate from E-owned prover logic.
- Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- The standalone PicoSAT app code owns signal handlers, alarm setup, decompressor subprocesses, and solver callbacks through file-static state. Rust should keep E's library-level SAT integration on an explicit solver object and avoid importing this process-global command-line behavior unless the vendored utility executable surface is intentionally ported.
- Compressed input handling is delegated to shell commands through `popen`, making diagnostics, quoting, and process cleanup platform-dependent. If these utilities are ever exposed by the Rust port, prefer an explicit decompression abstraction while preserving C compatibility only for reference comparison mode.
<!-- END MANUAL REVIEW: c_source_docs -->
