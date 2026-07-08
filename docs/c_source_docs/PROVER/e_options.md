<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_options

## Source Files

- [PROVER/e_options.h](../../../eprover/PROVER/e_options.h)

## Purpose

Options definitions and documentation. Moved here to reduce the size of the main eprover file. <1> Wed Aug 6 13:14:29 CEST 2014 New

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `OptionCodes`

### Macros And Constants

- `E_OPTIONS_GUARD`

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

- `E_OPTIONS_GUARD`
- `choice`
- `term`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_options.h`.

### Review Notes

- Reviewed as a standalone header unit in `PROVER` covering 1 source file(s), about 1759 lines, 1 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Command-line option declarations for `eprover`; keep flags, defaults, and help text consistent with the C binary.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Heuristic values are part of strategy behavior; preserve formulae, defaults, and parse names before optimizing.
- SAT/propositional integration has a separate assignment/result vocabulary; keep conversions and ownership boundaries explicit.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Rust Port Notes

- `src/prover/options.rs` mirrors the reviewed `eprover` option table as typed `OptCell` entries so command-line parsing can preserve C short names, long names, argument kinds, defaults, and visible help text while avoiding direct C enum exposure.

### Change Later

- The C `OptionCodes` enum and the `E_OPTIONS` table are maintained separately, so enum/table drift is possible and typoed identifiers such as `OPT_PRESAT_SIMPLIY` and `OPT_FW_SUMBSUMPTION_AGGRESSIVE` become compatibility names. Rust keeps typed option variants and C-shaped parsing, but any future generator should derive enum ids, option metadata, and help text from one source.
- Several C help strings contain visible historical typos and stale wording, including "peoblem", "deriviation", and old resource-limit platform notes. Preserve those where byte-compatible help output matters; a cleaned CLI should separate legacy help text from modern user-facing descriptions.
- Some options are compatibility aliases or no-ops in current executable paths because later `eprover.c` switch handling, feature gates, or parser surfaces decide whether they have effects. Keep the table exhaustive for parsing compatibility, but future non-drop-in configuration should expose only implemented behavior or report unsupported combinations structurally.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
