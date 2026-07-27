<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_rawspecfeatures

## Source Files

- [HEURISTICS/che_rawspecfeatures.h](../../../eprover/HEURISTICS/che_rawspecfeatures.h)
- [HEURISTICS/che_rawspecfeatures.c](../../../eprover/HEURISTICS/che_rawspecfeatures.c)

## Purpose

Code and datatypes for handling rough classification of raw problem specs. <1> Tue May 22 01:10:30 CEST 2012 New

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz (schulz@eprover.org)

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RawSpecFeatureCell`
- `RawSpecFeature_p`

### Macros And Constants

- `ADJUST_FOR_HO(limit, scale)`
- `NUM_RAW_FEATURES`
- `RAWSPECFEATURES`
- `RAW_CLASSIFY(index, value, some, many, ho_scale_some, ho_scale_many)`
- `RAW_CLASS_SIZE`

### Globals

- None found in the source scan.

### Exported Functions

- `void RawSpecFeaturesClassify(RawSpecFeature_p features, SpecLimits_p limits, char* pattern)`
- `void RawSpecFeaturesCompute(RawSpecFeature_p features, ProofState_p state)`
- `void RawSpecFeaturesParse(Scanner_p in, RawSpecFeature_p features)`
- `void RawSpecFeaturesPrint(FILE* out, RawSpecFeature_p features)`

## Implementation Notes

### Internal Functions

- None found in the source scan.

### Source-Level Behavior

- `RawSpecFeaturesCompute`: Compute the raw features of state.
- `RawSpecFeaturesClassify`: Add a classifiction based on limits to the (initialized) features.
- `RawSpecFeaturesParse`: Parse a rawspecfeatures line.
- `RawSpecFeaturesPrint`: Print the features.

### Dependencies

- `"che_rawspecfeatures.h"`
- `<che_clausesetfeatures.h>`

### Compile-Time Conditions

- `RAWSPECFEATURES`

## E Reference Notes

- Use the ownership model visible in this unit's allocation/free helpers and exported APIs as evidence; preserve it only where correctness, supported compatibility, or measured performance requires it.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions reveal performance assumptions; measure allocation granularity and reuse before choosing Umlaut's representation.
- Audit where clause/literal mutation order affects indexing, derivation, proof reconstruction, or deterministic behavior before changing it.
- Parser routines usually advance scanner state and may report fatal errors; preserve supported input behavior or document and test an intentional divergence.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_rawspecfeatures.h`, `HEURISTICS/che_rawspecfeatures.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 364 lines, 6 scanned public declarations, 0 scanned internal function definitions, and 4 structured function-comment blocks.
- Code and datatypes for handling rough classification of raw problem specs. <1> Tue May 22 01:10:30 CEST 2012 New
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Parser functions usually consume input and report fatal diagnostics on mismatch; exact token flow matters for compatibility.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.
- `RawSpecFeaturesCompute` combines clause-set cardinality, clause standard weight, clause conjecture/hypothesis counts, and signature symbol counts with formula-set cardinality/weight/counts, formula-set-only order, and active/archive formula definition statistics. C intentionally leaves `order` and `conj_order` at `1` for clause-only states even if clause terms are higher-order; Rust preserves that raw-spec compatibility surface while honoring owned formula sets.
- C measures each current clause or formula owner exactly once, including the `$true` formula wrappers used for type declarations. Rust parser metadata therefore adjusts this vector only when a formula had to be represented by lowered clauses; represented formula owners are measured directly from the formula set.
- The fallback-lowering adjustment is an ownership adapter rather than a second feature algorithm: Rust subtracts the generated clause count, standard weight, and conjecture/hypothesis roles before restoring the original formula count, weight, roles, order, lambda count, and applied-variable flag. A focused two-clause compensation regression plus byte-exact represented FOF/THF classifier comparisons are recorded in [`experiments/2026-07-17-083-rawspec-bridge-compensation/FINDINGS.md`](../../../experiments/2026-07-17-083-rawspec-bridge-compensation/FINDINGS.md). Replacing the remaining fallback parser belongs to the broader parser/formula-owner/CNF backlog, not to this computation unit.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.

### Change Later

- Once drop-in compatibility is secured, decide whether a cleaned classifier should expose clause-level higher-order order separately from the C raw-spec vector.
- Raw classification uses hard threshold buckets that feed directly into preprocessing-schedule selection, so a one-unit weight change at a boundary can select a substantially different prover configuration. A future strategy interface could retain the compatibility class while also exposing continuous feature values or explicit tie handling for less brittle schedule selection.

<!-- END MANUAL REVIEW: c_source_docs -->
