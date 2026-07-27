<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_kbdesc

## Source Files

- [LEARN/cle_kbdesc.h](../../../eprover/LEARN/cle_kbdesc.h)
- [LEARN/cle_kbdesc.c](../../../eprover/LEARN/cle_kbdesc.c)

## Purpose

Data types and functions for representing the knowledge base. the GNU Lesser General Public License. <1> Fri Jul 16 20:12:05 MET DST 1999 New

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `KBDescCell`
- `KBDesc_p`

### Macros And Constants

- `CLE_KB`
- `KBDescCellAlloc()`
- `KBDescCellFree(junk)`
- `KB_ANNOTATION_NO`
- `KB_VERSION`

### Globals

- None found in the source scan.

### Exported Functions

- `KBDesc_p KBDescAlloc(char* version, double neg_prop, long neg_examples)`
- `KBDesc_p KBDescParse(Scanner_p in)`
- `char* KBFileName(DStr_p name, char *basename, char* file)`
- `void KBDescFree(KBDesc_p desc)`
- `void KBDescPrint(FILE* out, KBDesc_p desc)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `KBDescAlloc`: Return an initialized KBDesc-Cell.
- `KBDescFree`: Free a KBDesc.
- `KBDescPrint`: Print a kb-description.
- `KBDescParse`: Parse a KB0Description.
- `KBFileName`: Build a kb-file name in name and return a pointer to it.

### Dependencies

- `"cle_kbdesc.h"`
- `"e_version.h"`
- `<cle_examplerep.h>`

### Compile-Time Conditions

- `CLE_KB`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_kbdesc.h`, `LEARN/cle_kbdesc.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 255 lines, 7 scanned public declarations, 0 scanned internal function definitions, and 5 structured function-comment blocks.
- Data types and functions for representing the knowledge base. the GNU Lesser General Public License. <1> Fri Jul 16 20:12:05 MET DST 1999 New
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Compatibility Notes

- `KBDescParse` compares the parsed version against `KB_VERSION` with plain `strcmp`, so "newer" is lexicographic string ordering rather than semantic version ordering.
- The "knowledge base is younger" diagnostic concatenates `" update from"` directly with `E_URL`, producing no space before the URL. Preserve the message shape unless user-facing diagnostics are intentionally cleaned up after compatibility is covered.
- `KBFileName` appends `"/"` between `basename` and `file` unconditionally; do not replace it with a platform path separator while learned KB file layout compatibility matters.
- `KBDescPrint` uses the build's `COMCHAR`; the default configured C checkout leaves `UNIX_COMMENTS` disabled, so generated KB description comments begin with `%`.

### Change Later

- Replace lexicographic version comparison with semantic version handling once old KB compatibility and migration behavior are defined.
- Make the comment-character policy explicit in KB file formats instead of inheriting it from the global `COMCHAR` build option.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
