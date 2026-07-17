<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# PCL2 / pcl_propanalysis

## Source Files

- [PCL2/pcl_propanalysis.h](../../../eprover/PCL2/pcl_propanalysis.h)
- [PCL2/pcl_propanalysis.c](../../../eprover/PCL2/pcl_propanalysis.c)

## Purpose

Functions for computing various properties of the clauses in a PCL protocol. the GNU Lesser General Public License. <1> Thu Feb 28 16:27:34 MET 2002

Within the source tree, this unit belongs to `PCL2`. PCL protocol and proof-object support: proof steps, mini-protocols, identifiers, positions, expressions, checking, lemmas, and proof analysis.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `PCLCmpFunType`
- `PCLPropDataCell`
- `PCLPropData_p`

### Macros And Constants

- `PCL_PROPANALYIS`

### Globals

- None found in the source scan.

### Exported Functions

- `PCLStep_p PCLProtFindMaxStep(PCLProt_p prot, PCLCmpFunType cmp)`
- `void PCLProtPropAnalyse(PCLProt_p prot, PCLPropData_p data)`
- `void PCLProtPropDataPrint(FILE* out, PCLPropData_p data)`

## Implementation Notes

### Internal Functions

- `pcl_depth_compare`
- `pcl_litno_compare`
- `pcl_prot_global_count`
- `pcl_sc_compare`
- `pcl_weight_compare`

### Source-Level Behavior

- `pcl_weight_compare`: Compare two PCL steps by standard weight of the clause.
- `pcl_sc_compare`: Compare two clause PCL steps by strict symbol count of the clause. FOF steps are always smaller and equivalent.
- `pcl_litno_compare`: Compare two PCL steps by literal number.
- `pcl_depth_compare`: Compare two PCL steps by clause depth.
- `pcl_prot_global_count`: Determine the global properties of the PCL listing.
- `PCLProtFindMaxStep`: Find and return the first PCL step from the protocol that is maximal with respect to cmp, NULL if prot is empty.
- `PCLProtPropAnalyse`: Analyse the PCL protocol and put the relevant information into data.
- `PCLProtPropDataPrint`: Print the result of the property analysis in reasonably readable form.

### Dependencies

- `"pcl_propanalysis.h"`
- `<che_clausefeatures.h>`
- `<pcl_protocol.h>`

### Compile-Time Conditions

- `PCL_PROPANALYIS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `PCL2/pcl_propanalysis.h`, `PCL2/pcl_propanalysis.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `PCL2` covering 2 source file(s), about 521 lines, 6 scanned public declarations, 5 scanned internal function definitions, and 8 structured function-comment blocks.
- Functions for computing various properties of the clauses in a PCL protocol. the GNU Lesser General Public License. <1> Thu Feb 28 16:27:34 MET 2002
- Proof-object code. Preserve identifier handling, step structure, protocol syntax, and proof-checking side effects.
- Clause and literal mutations can invalidate cached weights, indexes, or derivation metadata; keep update ordering visible.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Proof output/checking code is externally consumed; preserve identifiers, step ordering, and formatting details.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Rust Port Status

- Initial property-analysis support is ported as `src/pcl2/propanalysis.rs`, including max-step selection by standard weight, strict symbol count, literal count, and depth; protocol-wide FOF/positive/negative/mixed clause counts; literal and symbol-count aggregates; and C-shaped summary rendering through the ported `ClausePropInfoPrint` helper.
- Focused regressions now pin the complete boundary between aggregate counting and representative selection: empty clauses remain excluded from counts but eligible for all four maxima, FOF-only and shell-only protocols retain C's max-scan ordering, and zero denominators retain IEEE non-finite arithmetic. Rust makes empty, FOF-only, and shell-only rendering total instead of reproducing C's null or inactive-union dereferences. Archived `epclanalyse` terminates with `SIGSEGV` on the first two corpora, while the permanent safe-boundary executable case remains exact; [`experiment 064`](../../../experiments/2026-07-16-064-pcl-propanalysis-edges/FINDINGS.md) records the evidence.

### Change Later

- `PCLProtPropDataPrint` divides by clause-category counts without zero guards and then unconditionally prints selected max-step pointers. Empty protocols, FOF-only protocols, or protocols with no positive/negative/mixed clauses can therefore produce infinities/NaNs or dereference invalid logical-content union arms. Rust keeps the C-shaped average formulas but avoids invalid dereferences; after drop-in compatibility is secured, this reporting path should get explicit "not available" output.
- `pcl_prot_global_count` excludes empty clauses from all aggregate clause and literal counts, while `PCLProtFindMaxStep` can still select empty clauses for max-step fields because it scans all non-FOF steps. Rust preserves that split; a later reporting API should decide whether empty clauses are statistical clauses or just proof sentinels.
- The C comparators treat FOF steps as smaller and equivalent but assume every non-FOF step has a clause in the `logic` union. Shell PCL steps violate that assumption. Rust treats shell steps as zero-metric non-FOF steps; the focused core regression preserves selection and rendering without reading the inactive union arm. C `epclanalyse` rejects shell PCL during parsing, so this remains a source-level internal-API boundary rather than an executable comparison case.
<!-- END MANUAL REVIEW: c_source_docs -->
