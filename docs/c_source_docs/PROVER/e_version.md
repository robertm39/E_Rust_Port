<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PROVER / e_version

## Source Files

- [PROVER/e_version.h](../../../eprover/PROVER/e_version.h)

## Purpose

Define global macros for version number and meta-information. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `PROVER`. Command-line programs and top-level prover/server/client tools that assemble library modules into executable workflows.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTR_COPYRIGHT`
- `E_FOOTER`
- `E_NICKNAME`
- `E_URL`
- `E_VERSION`
- `HO_MAIL`
- `LFH`
- `PVERSION`
- `STS_COPYRIGHT`
- `STS_MAIL`
- `STS_SNAIL`
- `VERSION`

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

- `"e_gitcommit.h"`

### Compile-Time Conditions

- `ENABLE_LFHO`
- `E_VERSION`
- `NDEBUG`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Preserve compile-time feature gates and debug-only behavior as explicit Rust configuration or narrowly scoped runtime options.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PROVER/e_version.h`.

### Review Notes

- Reviewed as a standalone header unit in `PROVER` covering 1 source file(s), about 111 lines, 0 scanned public declarations, 0 scanned internal function definitions, and 0 structured function-comment blocks.
- Version/build metadata surface. Rust replacement should expose compatible version and build identifiers.
- Executable entry-point code. These files define command-line compatibility and compose the libraries into user-visible tools.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
