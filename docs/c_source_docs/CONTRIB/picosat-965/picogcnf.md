<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CONTRIB/picosat-965 / picogcnf

## Source Files

- [CONTRIB/picosat-965/picogcnf.c](../../../../eprover/CONTRIB/picosat-965/picogcnf.c)

## Purpose

PicoSAT utility source for grouped CNF workflows.

Within the source tree, this unit belongs to `CONTRIB/picosat-965`. Vendored PicoSAT SAT-solver sources used through E's propositional/SAT integration paths. These files follow PicoSAT's API and allocation conventions.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `LOG(ARGS...)`

### Globals

- None found in the source scan.

### Exported Functions

- `main`

## Implementation Notes

### Internal Functions

- `callback`
- `die`
- `msg`
- `percent`

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- `"picosat.h"`
- `<assert.h>`
- `<ctype.h>`
- `<limits.h>`
- `<stdarg.h>`
- `<stdio.h>`

### Compile-Time Conditions

- None found in the source scan.

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CONTRIB/picosat-965/picogcnf.c`.

### Review Notes

- Reviewed as a standalone C implementation unit in `CONTRIB/picosat-965` covering 1 source file(s), about 166 lines, 0 scanned public declarations, 4 scanned internal function definitions, and 0 structured function-comment blocks.
- PicoSAT utility for grouped CNF handling; preserve only if the vendored tool surface is ported.
- Vendored PicoSAT code. Keep the boundary explicit: document API expectations and integration points, but avoid blending PicoSAT implementation assumptions into E-owned Rust modules.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- The grouped-CNF utility is a standalone PicoSAT workflow with its own parser, progress messaging, and fatal-error path. Rust should not blend this utility parser into E's clause parser; port it only if the external vendored utility surface becomes part of the compatibility target.
- Progress percentages and grouped-clause callbacks are presentation-side concerns interleaved with parsing and solving. A cleaned Rust utility could separate parsing, solver callbacks, and output formatting after byte-for-byte utility compatibility is no longer required.
<!-- END MANUAL REVIEW: c_source_docs -->
