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
<!-- END MANUAL REVIEW: c_source_docs -->
