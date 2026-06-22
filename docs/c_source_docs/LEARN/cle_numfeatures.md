<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# LEARN / cle_numfeatures

## Source Files

- [LEARN/cle_numfeatures.h](../../../eprover/LEARN/cle_numfeatures.h)
- [LEARN/cle_numfeatures.c](../../../eprover/LEARN/cle_numfeatures.c)

## Purpose

Functions and data types for dealing with numerical features of the clause set. This is, unfortunatly, not quite orthogonal to che_clausesetfeatures.h at the moment. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `LEARN`. Learning and knowledge-base support: example representations, feature extraction, term-space maps, annotations, and KB insertion/description helpers.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `FeaturesCell`
- `Features_p`

### Macros And Constants

- `CHE_NUMFEATURES`
- `FEATURE_NUMBER`
- `FeaturesCellAlloc()`
- `FeaturesCellFree(junk)`
- `SEL_FEATURE_WEIGHTS`
- `SEL_FUNC_WEIGHT`
- `SEL_PRED_WEIGHT`

### Globals

- None found in the source scan.

### Exported Functions

- `Features_p FeaturesAlloc(void)`
- `Features_p NumFeaturesParse(Scanner_p in)`
- `double NumFeatureDistance(Features_p f1, Features_p f2, double pred_w, double func_w, double* weights)`
- `void ComputeClauseSetNumFeatures(Features_p features, ClauseSet_p set, Sig_p sig)`
- `void FeaturesFree(Features_p junk)`
- `void NumFeaturesPrint(FILE* out, Features_p features)`

## Implementation Notes

### Internal Functions

- `arity_distr_distance`
- `parse_sig_distrib`
- `relative_difference`

### Source-Level Behavior

- `relative_difference`: Return the relative difference of two values.
- `arity_distr_distance`: Compute the normed euclidean distance beween two arity distribution vectors.
- `parse_sig_distrib`: Parse a list (n0, n1, ... nn) into a PDArray.
- `FeaturesAlloc`: Allocate an empty, initialized FeaturesCell()
- `FeaturesFree`: Free a FeaturesCell()
- `ComputeClauseSetNumFeatures`: Compute the numerical features of a clause set. This is not as modular as I would have liked, as I expect this to be done fairly often and hence want to do it in a single pass.
- `NumFeaturesPrint`: Print the feature cell.
- `NumFeaturesParse`: Parse a set of features.
- `NumFeatureDistance`: Return the weighted relative distance between the two feature vectors.

### Dependencies

- `"cle_numfeatures.h"`
- `<ccl_clausesets.h>`

### Compile-Time Conditions

- `CHE_NUMFEATURES`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
- Parser routines usually advance scanner state and may report fatal errors; keep token-consumption behavior exact.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `LEARN/cle_numfeatures.h`, `LEARN/cle_numfeatures.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `LEARN` covering 2 source file(s), about 565 lines, 8 scanned public declarations, 3 scanned internal function definitions, and 9 structured function-comment blocks.
- Functions and data types for dealing with numerical features of the clause set. This is, unfortunatly, not quite orthogonal to che_clausesetfeatures.h at the moment. the GNU Lesser General Public License.
- Learning support code. Keep feature-vector layout, term-space-map behavior, and KB I/O stable enough for existing learned data.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Container use is pointer-oriented and often encodes ownership by convention rather than type; map this to Rust lifetimes/owners deliberately.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
