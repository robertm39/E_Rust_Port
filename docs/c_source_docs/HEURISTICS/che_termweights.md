<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# HEURISTICS / che_termweights

## Source Files

- [HEURISTICS/che_termweights.h](../../../eprover/HEURISTICS/che_termweights.h)
- [HEURISTICS/che_termweights.c](../../../eprover/HEURISTICS/che_termweights.c)

## Purpose

Common functions for term-based clause evaluation heuristics. the GNU Lesser General Public License.

Within the source tree, this unit belongs to `HEURISTICS`. Clause evaluation and strategy machinery: heuristic control blocks, priority functions, weight functions, literal selection, automatic strategy selection, and feature extraction.

Authors noted in source headers: Stephan Schulz, yan

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `RelatedTermSet`

### Macros And Constants

- `CHE_TERMWEIGHTS`
- `MIN3(a, b, c)`
- `TERM_MAX_GENS`

### Globals

- None found in the source scan.

### Exported Functions

- `NumTree_p TBCountTermFreqs(TB_p bank)`
- `PStack_p ComputeSubtermsGeneralizations(Term_p term, VarBank_p vars)`
- `PStack_p ComputeTopGeneralizations( Term_p term, VarBank_p vars, Sig_p sig)`
- `int TupleInit(FixedDArray_p cur)`
- `int TupleNext(FixedDArray_p cur, FixedDArray_p max)`
- `void FreeGeneralizations(PStack_p gens)`
- `void TBIncSubtermsFreqs(Term_p term, NumTree_p* freqs)`
- `void TuplePrint(FixedDArray_p t)`

## Implementation Notes

### Internal Functions

- `compute_subterms_generalizations`
- `get_subterm_generalizing_vars`

### Source-Level Behavior

- `ComputeSubtermsGeneralizations`: Compute generalizations of all subterms. The number of gens per term is limited by TERM_MAX_GENS.
- `ComputeTopGeneralizations`: Compute top-level generalization "f(X1,..,Xn)" for each n-ary f from the term.
- `FreeGeneralizations`: Free the stack of terms and their top symbols.
- `TupleInit`: Used to traverse n-tupples from (0,0,...,0) to (n0,n1,...,nm) lexicographically. This sets the tupple to zeros.
- `TupleNext`: Used to traverse n-tupples from (0,0,...,0) to (n0,n1,...,nm) lexicographically. This makes `cur` the next tupple. Maximal values for each item (that is, those n's) is given in `max`.
- `TuplePrint`: Print a tuple.
- `TBIncSubtermsFreqs`: Increase the frequency of all subterms of `term` in `freqs` where `freqs` is a map from Term_p.entry_no to the frequency.
- `TBCountTermFreqs`: Iterates over a term bank and set the number of occurences (frequencies) for each term. Only top positions are processed but other terms are also visited as all subterms are considered.

### Dependencies

- `"che_termweights.h"`
- `<ccl_relevance.h>`
- `<che_refinedweight.h>`

### Compile-Time Conditions

- `CHE_TERMWEIGHTS`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `HEURISTICS/che_termweights.h`, `HEURISTICS/che_termweights.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `HEURISTICS` covering 2 source file(s), about 549 lines, 9 scanned public declarations, 2 scanned internal function definitions, and 8 structured function-comment blocks.
- Common functions for term-based clause evaluation heuristics. the GNU Lesser General Public License.
- Strategy code. Preserve clause-evaluation semantics, priority ordering, weight formulas, and feature extraction because small changes can alter search behavior.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
