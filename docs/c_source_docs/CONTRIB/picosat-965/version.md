<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTRIB/picosat-965 / version

## Source Files

- [CONTRIB/picosat-965/version.c](../../../../eprover/CONTRIB/picosat-965/version.c)

## Purpose

PicoSAT version metadata source.

Within the source tree, this unit belongs to `CONTRIB/picosat-965`. Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- None found in the source scan.

### Globals

- None found in the source scan.

### Exported Functions

- `picosat_config`
- `picosat_version`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"config.h"`

### Compile-Time Conditions

- None found in the source scan.

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTRIB/picosat-965/version.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `CONTRIB/picosat-965` covering 1 source file(s), about 15 lines, 0 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- PicoSAT version metadata source for vendored utility builds.
- Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
