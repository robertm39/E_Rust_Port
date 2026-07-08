<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_gitcommit

## Source Files

- [PROVER/e_gitcommit.h](../../../eprover/PROVER/e_gitcommit.h)

## Purpose

e_gitcommit is a standalone header in PROVER.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `ECOMMITID`

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

- None found in the source scan.

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_gitcommit.h`.

### Review Notes

- Reviewed as a standalone header unit in `PROVER` covering 1 source file(s), about 2 lines, 0 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- `e_gitcommit` provides the `e gitcommit` part of the `PROVER` subsystem.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.

### Rust Port Notes

- `src/prover/version.rs` mirrors the static upstream `ECOMMITID` string so `--version` output can match the reviewed C source snapshot.

### Change Later

- `ECOMMITID` identifies the upstream E source commit baked into this checkout, not the Rust port's current git commit. Keep that for drop-in `--version` compatibility, but a cleaned build-info surface should expose upstream-source and Rust-port revisions as separate fields.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
