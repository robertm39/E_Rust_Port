<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_patterns

## Source Files

- [HEURISTICS/che_patterns.h](../../../eprover/HEURISTICS/che_patterns.h)

## Purpose

Routines for dealing with term patterns. the GNU Lesser General Public License. <1> Sat Jul 5 02:28:25 MET DST 1997 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLE_PATTERNS`

### Globals

- None found in the source scan.

### Exported Functions

- None found in the source scan.

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- No structured function-comment blocks were found; rely on the declaration lists and direct source review.

### Dependencies

- None found in the source scan.

### Compile-Time Conditions

- `CLE_PATTERNS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_patterns.h`.

### Review Notes

- Reviewed as a standalone header unit in `HEURISTICS` covering 1 source file(s), about 54 lines, 0 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Routines for dealing with term patterns. the GNU Lesser General Public License. <1> Sat Jul 5 02:28:25 MET DST 1997 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
