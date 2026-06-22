<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTRIB/picosat-965 / picomcs

## Source Files

- [CONTRIB/picosat-965/picomcs.c](../../../../eprover/CONTRIB/picosat-965/picomcs.c)

## Purpose

PicoSAT utility source for minimal correction set workflows.

Within the source tree, this unit belongs to `CONTRIB/picosat-965`. Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `Clause`
- `MCS`

### Macros And Constants

- None found in the source scan.

### Globals

- None found in the source scan.

### Exported Functions

- `dump`
- `if`
- `main`

## Implementation Notes

### Internal Functions

- `camcs`
- `clause2selvar`
- `cumcs`
- `cumcscb`
- `dump_clause`
- `encode`
- `encode_clause`
- `msg`
- `nextch`
- `parse`
- `print_all_mcs`
- `print_mcs`
- `print_umcs`
- `push_clause`
- `push_mcs`
- `push_stack`
- `release`
- `release_clauses`
- `release_mss`

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"picosat.h"`
- `<assert.h>`
- `<ctype.h>`
- `<stdarg.h>`
- `<stdio.h>`
- `<stdlib.h>`
- `<string.h>`
- `<unistd.h>`

### Compile-Time Conditions

- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTRIB/picosat-965/picomcs.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `CONTRIB/picosat-965` covering 1 source file(s), about 335 lines, 2 scanned public declarations, 19 scanned internal function definitions, and 0 structured function-comment blocks.
- PicoSAT minimal correction set utility; keep this utility distinct from E's library-level SAT calls.
- Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
