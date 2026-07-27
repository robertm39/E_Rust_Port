<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_examplerep

## Source Files

- [LEARN/cle_examplerep.h](../../../eprover/LEARN/cle_examplerep.h)
- [LEARN/cle_examplerep.c](../../../eprover/LEARN/cle_examplerep.c)

## Purpose

Data structures and functions to associate names, numbers and features with a proof problem. the GNU Lesser General Public License. <1> Mon Jul 26 18:30:59 MET DST 1999

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `ExampleRepCell`
- `ExampleRep_p`
- `ExampleSetCell`
- `ExampleSet_p`

### Macros And Constants

- `CLE_EXAMPLEREP`
- `ExampleRepCellAlloc()`
- `ExampleRepCellFree(junk)`
- `ExampleSetCellAlloc()`
- `ExampleSetCellFree(junk)`

### Globals

- None found in the source scan.

### Exported Functions

- `ExampleRep_p ExampleRepParse(Scanner_p in)`
- `ExampleRep_p ExampleSetExtract(ExampleSet_p set, ExampleRep_p rep)`
- `ExampleRep_p ExampleSetFindName(ExampleSet_p set, char* name)`
- `ExampleSet_p ExampleSetAlloc(void)`
- `bool ExampleSetDeleteId(ExampleSet_p set, long ident)`
- `bool ExampleSetDeleteName(ExampleSet_p set, char* name)`
- `bool ExampleSetInsert(ExampleSet_p set, ExampleRep_p rep)`
- `long ExampleSetParse(Scanner_p in, ExampleSet_p set)`
- `long ExampleSetSelectByDist(PStack_p results, ExampleSet_p set, Features_p target, double pred_w, double func_w, double *weights, long sel_no, double set_part, double dist_part)`
- `void ExampleRepFree(ExampleRep_p junk)`
- `void ExampleRepPrint(FILE* out, ExampleRep_p rep)`
- `void ExampleSetFree(ExampleSet_p junk)`
- `void ExampleSetPrint(FILE* out, ExampleSet_p set)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `ExampleRepFree`: Free an example represenation.
- `ExampleRepPrint`: Print an example representation.
- `ExampleRepParse`: Parse an example representation and return a pointer to it.
- `ExampleSetAlloc`: Allocate an empty example set and return a pointer to it.
- `ExampleSetFree`: Free an exampel set.
- `ExampleSetFindName`: Find an entry by name, return NULL if non-existant.
- `ExampleSetInsert`: Insert rep into set. Return true if it works, false otherwise.
- `ExampleSetExtract`: Extract rep from set and return it. Return NULL is rep does not exist in set.
- `ExampleSetDeleteId`: Delete the example with ident id. Returns success.
- `ExampleSetDeleteName`: Delete the example with name name. Returns success.
- `ExampleSetPrint`: Print a set of example representations.
- `ExampleSetParse`: Parse a list of examples into set. Return number of items parsed.
- `ExampleSetSelectByDist`: Push idents of the most similar examples onto results. How many examples is controlled by select, set_part, and dist_part: Selected are at most select examples, at most part*setsize examples and only examples whose distance is not larger than dist_part times average distance.

### Dependencies

- `"cle_examplerep.h"`
- `<clb_simple_stuff.h>`
- `<clb_stringtrees.h>`
- `<cle_numfeatures.h>`

### Compile-Time Conditions

- `CLE_EXAMPLEREP`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_examplerep.h`, `LEARN/cle_examplerep.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 564 lines, 17 scanned public declarations, 0 scanned internal function definitions, and 13 structured function-comment blocks.
- Data structures and functions to associate names, numbers and features with a proof problem. the GNU Lesser General Public License. <1> Mon Jul 26 18:30:59 MET DST 1999
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `ExampleSetInsert` stores the representation in the numeric `ident_index` before inserting the name in `name_index`. If the name insert fails because another example already uses that name, the function returns false after the numeric tree has already been changed; Rust preserves this side effect in the already-ported helper tests.
- `set->count` is a high-water mark updated only on successful insert and is not decremented by `ExampleSetDeleteId` or `ExampleSetDeleteName`. This matters for later generated identifiers and for matching existing learned-data numbering.
- `ExampleSetPrint` iterates the numeric tree, so output order follows example id rather than parse order or name order.

### Change Later

- A modernized example set should make insertion atomic across both indexes or report partial insertion explicitly. The current C behavior can leave the two indexes inconsistent after duplicate-name insertion.
- Rename or split the `count` field if the API is cleaned up; it is not the current set size, but the maximum successfully inserted identifier.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
