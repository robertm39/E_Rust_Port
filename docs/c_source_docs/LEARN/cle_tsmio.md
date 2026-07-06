<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_tsmio

## Source Files

- [LEARN/cle_tsmio.h](../../../eprover/LEARN/cle_tsmio.h)
- [LEARN/cle_tsmio.c](../../../eprover/LEARN/cle_tsmio.c)

## Purpose

Functions for building TSMs from a knowledge base. the GNU Lesser General Public License. <1> Tue Aug 31 13:23:14 MET DST 1999 New

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CLE_TSMIO`

### Globals

- None found in the source scan.

### Exported Functions

- `TSMAdmin_p TSMFromKB(bool flat_patterns, double evalweights[], char* kb, Sig_p sig, ClauseSet_p target, long sel_no, double set_part, double dist_part, IndexType indextype, TSMType tsmtype, long indexdepth)`
- `double ExampleSetFromKB(AnnoSet_p annoset, FlatAnnoSet_p flatset, bool flat_patterns, TB_p bank, double evalweights[], char* kb, Sig_p sig, ClauseSet_p target, long sel_no, double set_part, double dist_part)`
- `double ExampleSetPrepare(FlatAnnoSet_p flatset, AnnoSet_p annoset, double evalweights[], ExampleSet_p examples, Sig_p sig, ClauseSet_p target, long sel_no, double set_part, double dist_part)`

## Implementation Notes

### Internal Functions

- `get_default_eval`
- `get_highest_weight`
- `level_get_highest_weight`
- `rec_get_highest_weight`

### Source-Level Behavior

- `get_default_eval`: Return the default evaluation for a set of annoterms: Assume proofs=0, proof distance = max pd in set+1, all other values = average.
- `rec_get_highest_weight`: Return the highest eval_weight in a recursive tsm.
- `level_get_highest_weight`: Return the highest eval_weight in a sinlge-level tsm.
- `get_highest_weight`: Return the highest eval_weight in a tsm.
- `ExampleSetPrepare`: Create a flat annotated example set with examples tailored to target from annoset. Return the default evaluatiopn.
- `ExampleSetFromKB`: Create a flat annotated example set from a knowledge base. Return default evaluation.
- `TSMFromKB`: Create a tsm for evaluating clauses for proving target from kb

### Dependencies

- `"cle_tsmio.h"`
- `<cle_examplerep.h>`
- `<cle_kbdesc.h>`
- `<cle_tsm.h>`

### Compile-Time Conditions

- `CLE_TSMIO`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_tsmio.h`, `LEARN/cle_tsmio.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 444 lines, 3 scanned public declarations, 4 scanned internal function definitions, and 7 structured function-comment blocks.
- Functions for building TSMs from a knowledge base. the GNU Lesser General Public License. <1> Tue Aug 31 13:23:14 MET DST 1999 New
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- File-static state should be audited for thread-safety and reset behavior in the Rust port.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `get_default_eval` sets the temporary annotation length to `KB_ANNOTATION_NO` (`7`) and loops over slots `3..=7`, but `AnnotationEval` evaluates only slots `1..6` for length `7`. Slot `7` is accumulated and normalized but ignored in the returned default evaluation.
- `get_default_eval` stores `AnnotationCount` in a C `long`, so fractional counts are truncated before they are used as weights and before the total count divisor is updated.
- `ExampleSetPrepare` declares its local `res` as `long` even though `get_default_eval` and the exported function return `double`; any fractional default evaluation is truncated before return.
- `ExampleSetFromKB` opens `signature` and `problems` with comment skipping enabled, mutates the supplied signature from the signature file, optionally recodes the supplied annotation set from recursive to flat clause encoding, and then delegates all selection/flattening/normalization work to `ExampleSetPrepare`.
- `rec_get_highest_weight` and `level_get_highest_weight` initialize their result to `1000000000000.0` and then take `MAX` with all actual `eval_weight` values. As written, they return the large sentinel rather than the true highest training weight. Preserve this for `TSMFromKB` parity, but revisit the unmapped-weight policy behind learned-map reference tests.
- `TSMFromKB` parses `clausepatterns` before loading the KB `signature` file because the temporary C term bank shares the caller's raw `Sig_p`. A Rust port with owned term-bank signatures needs an explicit signature synchronization step or a future shared-signature owner.
- `TSMFromKB` emits `VERBOUT("TSM created\n")` after successful construction. Rust now preserves this through the global verbose wrapper on the public path and an injected-writer helper for tests.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
