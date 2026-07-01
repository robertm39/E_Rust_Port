<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_positions

## Source Files

- [PCL2/pcl_positions.h](../../../eprover/PCL2/pcl_positions.h)
- [PCL2/pcl_positions.c](../../../eprover/PCL2/pcl_positions.c)

## Purpose

Positions in PCL2 clauses. the GNU Lesser General Public License. <1> Wed Mar 22 19:32:20 MET 2000 New

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCL2PosCell`
- `PCL2Pos_p`

### Macros And Constants

- `PCL2PosCellAlloc()`
- `PCL2PosCellFree(junk)`
- `PCL_POSITIONS`

### Globals

- None found in the source scan.

### Exported Functions

- `PCL2Pos_p PCL2PosAlloc(void)`
- `PCL2Pos_p PCL2PosParse(Scanner_p in)`
- `void PCL2PosFree(PCL2Pos_p pos)`
- `void PCL2PosPrint(FILE* out, PCL2Pos_p pos)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `PCL2PosAlloc`: Allocate an initialized PCL2 position data structure.
- `PCL2PosFree`: Free a PCL2 position.
- `PCL2PosParse`: Parse a PCL2 position of the format <pos-int> [. L|R [ .<pos-int> ]*].
- `PCL2PosPrint`: Print a PCL2 position.

### Dependencies

- `"pcl_positions.h"`
- `<ccl_eqn.h>`

### Compile-Time Conditions

- `PCL_POSITIONS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_positions.h`, `PCL2/pcl_positions.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 256 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Positions in PCL2 clauses. the GNU Lesser General Public License. <1> Wed Mar 22 19:32:20 MET 2000 New
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status Notes

- `src/pcl2/positions.rs` ports `PCL2PosAlloc`, `PCL2PosParse`, and `PCL2PosPrint` with the existing Rust scanner and the ported `EqnSide` discriminants from `ccl_eqn`.
- The Rust representation stores the optional term path in a `Vec<i64>` instead of a nullable `PDArray`; this preserves the observable empty/non-empty position state without exposing the C allocation sentinel.
- The Rust printer intentionally preserves the C separator behavior: parsed input such as `3.L.4.5` renders as `3.L45`, because `PCL2PosPrint` prints term-position components without a preceding full stop.

### Change Later

- `PCL2PosPrint` omits separators before term-position components even though `PCL2PosParse` requires dotted components. That makes multi-component printed positions fail to round-trip through the parser. Keep this for compatibility until reference PCL traces say whether external tools depend on it.
- The header comment says the literal and side are optional, but `PCL2PosParse` always starts by accepting a positive integer literal. Revisit the syntax contract when the rest of PCL2 proof-object parsing is ported.
- C allocates `PDArrayAlloc(5,10)` only when at least one term-position component follows the side. Rust uses `Vec<i64>` for now; if PCL position parsing becomes hot in proof checking, benchmark whether the C small-array growth shape matters.
<!-- END MANUAL REVIEW: c_source_docs -->
