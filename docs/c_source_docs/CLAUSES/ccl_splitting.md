<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# CLAUSES / ccl_splitting

## Source Files

- [CLAUSES/ccl_splitting.h](../../../eprover/CLAUSES/ccl_splitting.h)
- [CLAUSES/ccl_splitting.c](../../../eprover/CLAUSES/ccl_splitting.c)

## Purpose

Implements functions for destructive splitting of clauses with at least two non-propositional variable disjoint subsets of literals. the GNU Lesser General Public License. <1> Wed Apr 18 18:24:18 MET DST 2001

Within the source tree, this unit belongs to `CLAUSES`. Clause, formula, literal, inference, simplification, indexing, subsumption, grounding, relevance, and proof-state machinery.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `LitSplitDescCell`
- `LitSplitDesc_p`
- `SplitClassType`
- `SplitType`

### Macros And Constants

- `CCL_SPLITTING`
- `QuerySplitClass(var, prop)`
- `SetSplitClass(var, prop)`

### Globals

- None found in the source scan.

### Exported Functions

- `bool ClauseHasSplitLiteral(Clause_p clause)`
- `int ClauseSplit(DefStore_p store, Clause_p clause, ClauseSet_p set, SplitType how, bool fresh_defs)`
- `long ClauseSetSplitClauses(DefStore_p store, ClauseSet_p from_set, ClauseSet_p to_set, SplitType how, bool fresh_defs)`
- `long ClauseSetSplitClausesGeneral(DefStore_p store, bool fresh_defs, ClauseSet_p from_set, ClauseSet_p to_set, long tries)`

## Implementation Notes

### Internal Functions

- `build_part`
- `cond_init_lit_table`
- `find_free_literal`
- `initialize_lit_table`
- `initialize_permute_stack`
- `permute_stack_next`

### Source-Level Behavior

- `initialize_lit_table`: Initialize the literal table. For each literal, mark them as unassigned to any art and collect the variables that are marked by var_filter. If ground literals are not split off individually, assign them to partition 1.
- `cond_init_lit_table`: Initialize the literal table. For each literal, mark them as unassigned to any art and collect the variables that are marked by var_filter. If ground literals are not split off individually, assign them to partition 1.
- `find_free_literal`: Find the first entry in lit_table that corresponds to a literal not yet assigned to any clause part and return its index. If none exists, return -1.
- `build_part`: Given the index of the first unassigned literal in lit_table and a part number, assign this number to all literals that are transitively variable-linked to this first literal.
- `assemble_part_literals`: Given a partition number, assemble and return all literals belonging to that partition.
- `clause_split_general`: Try to split clause into different clauses according to the inference rule below. If successful, deposit split clauses into set and return number of clauses created. Otherwise return 0. L1(X) v L2(X) v L3(X) ... T1(X) v T2(X) v T3(X) ...., ~T1(X) v L1(X), ~T2(X) v L2(X), ~T3(X) v L3(X), ... if the Li are subsets of the clause that do not share any variables...
- `initialize_permute_stack`: We want to generate unordered n-tuples from k elements. This initializes a stack to contain the first valid sample (1, 2, ...n) of size n.
- `permute_stack_next`: Generate the next valid permutation and return true if it exists, otherwise return false.
- `ClauseHasSplitLiteral`: Return true if a literal in the clause is a split literal.
- `ClauseSplit`: Try to split clause into different clauses according to the inference rule below. If successful, deposit split clauses into set and return number of clauses created. Otherwise return 0. L1 v L2 v L3 ... T1 v T2 v T3 ...., ~T1 v L1, ~T2 v L2, ~T3 v L3, ... if the Li are variable-disjoint subsets of the clause and the Ti are _new_ propositional variables.
- `ClauseSplitGeneral`: Wrapper for clause_split_general(). Tries tries different variable subsets (partially ordered by cardinality) to find a subset that splits the clause. Only used for eground, so I skimp on options.
- `ClauseSetSplitClauses`: Split all clauses in from_set and put the result into to_set.
- `ClauseSetSplitClausesGeneral`: Split all clauses in from_set and put the result into to_set.

### Dependencies

- `"ccl_splitting.h"`
- `<ccl_def_handling.h>`

### Compile-Time Conditions

- `CCL_SPLITTING`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Allocation helpers and paired free functions are part of the performance contract; keep allocation granularity and reuse behavior visible in the Rust design.
- Container APIs often transfer raw pointers without ownership annotations; document and encode ownership at the Rust boundary.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
- Clause/literal mutation affects indexing, derivation, and proof reconstruction; preserve update ordering.
<!-- END AUTO-GENERATED: c_source_docs -->








<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `CLAUSES/ccl_splitting.h`, `CLAUSES/ccl_splitting.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `CLAUSES` covering 2 source file(s), about 788 lines, 8 scanned public declarations, 6 scanned internal function definitions, and 13 structured function-comment blocks.
- Implements functions for destructive splitting of clauses with at least two non-propositional variable disjoint subsets of literals. the GNU Lesser General Public License. <1> Wed Apr 18 18:24:18 MET DST 2001
- Clause/formula code. Pay close attention to mutation ordering because indexes, derivation records, and proof-state sets are updated around the same objects.
- Memory ownership is explicit in the C API; identify which returned pointers are owned by the caller and which are borrowed/shared before porting.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
