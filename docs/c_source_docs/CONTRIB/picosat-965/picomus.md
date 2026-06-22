<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTRIB/picosat-965 / picomus

## Source Files

- [CONTRIB/picosat-965/picomus.c](../../../../eprover/CONTRIB/picosat-965/picomus.c)

## Purpose

PicoSAT utility source for minimal unsatisfiable subset workflows.

Within the source tree, this unit belongs to `CONTRIB/picosat-965`. Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `Cls`

### Macros And Constants

- `MAXCOREROUNDS`
- `MAXNONREDROUNDS`
- `MINCOREROUNDS`

### Globals

- None found in the source scan.

### Exported Functions

- `main`

## Implementation Notes

### Internal Functions

- `callback`
- `die`
- `msg`
- `next`
- `parse`
- `percent`
- `warn`

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"picosat.h"`
- `<assert.h>`
- `<ctype.h>`
- `<stdarg.h>`
- `<stdio.h>`
- `<string.h>`

### Compile-Time Conditions

- `NDEBUG`
- `TRACE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTRIB/picosat-965/picomus.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `CONTRIB/picosat-965` covering 1 source file(s), about 408 lines, 1 scanned public declarations, 7 scanned internal function definitions, and 0 structured function-comment blocks.
- PicoSAT MUS utility; useful as vendored context, not as a core E module.
- Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
