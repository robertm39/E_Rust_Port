<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# TERMS / cte_termtrees

## Source Files

- [TERMS/cte_termtrees.h](../../../eprover/TERMS/cte_termtrees.h)
- [TERMS/cte_termtrees.c](../../../eprover/TERMS/cte_termtrees.c)

## Purpose

Functionality of term-top indexed trees (I found that I can cleanly separate this from the termbank stuff). There are two sets of funktions for the manangment of term trees in CLIB: Funktions operating only on the top cell, and functions descending the term structure. Top level functions implement a conventional AVL tree with key f_code.masked_properties.entry_nos_of_args and are

Within the source tree, this unit belongs to `TERMS`. Typed term representation and manipulation: signatures, term banks, substitutions, matching/unification, higher-order/lambda handling, variable sets, types, and term indexing.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- None found in the source scan.

### Macros And Constants

- `CTE_TERMTREES`
- `TermTreeTraverseExit(stack)`

### Globals

- None found in the source scan.

### Exported Functions

- `Term_p TermTreeExtract(Term_p *root, Term_p term)`
- `Term_p TermTreeFind(Term_p *root, Term_p term)`
- `Term_p TermTreeInsert(Term_p *root, Term_p term)`
- `bool TermTreeDelete(Term_p *root, Term_p term)`
- `long TermTopCompare(Term_p t1, Term_p t2)`
- `long TermTreeNodes(Term_p root)`
- `void TermTreeDelProp(Term_p root, TermProperties props)`
- `void TermTreeFree(Term_p junk)`
- `void TermTreeSetProp(Term_p root, TermProperties props)`

## Implementation Notes

### Internal Functions

- `splay_term_tree`

### Source-Level Behavior

- `splay_tree`: Perform the splay operation on tree at node with key.
- `TermTreeFree`: Release the memory taken by a term top AVL tree. Do not free variables, as they belong to a variable bank as well. Yes, this is an ugly hack! *sigh*
- `TermTopCompare`: Compare two top level term cells as f_code.masked_properties.args_as_pointers, return a value >0 if t1 is greater, 0 if the terms are identical, <0 if t2 is greater.
- `TermTreeFind`: Find a entry in the term tree, given a cell with correct (i.e. term-bank) argument pointers. pointers
- `TermTreeInsert`: Insert a term with valid subterm pointers into the termtree. If the entry already exists, return pointer to existing entry as usual, otherwise return NULL.
- `TermTreeExtract`: Remove a top term cell from the term tree and return a pointer to it.
- `TermTreeDelete`: Delete a top term from the term tree.
- `TermTreeSetProp`: Set the given properties for all term cells in the tree.
- `TermTreeDelProp`: Delete the given properties for all term cells in the tree.
- `TermTreeNodes`: Return the number of nodes in the tree.

### Dependencies

- `"cte_termtrees.h"`
- `<cte_termfunc.h>`

### Compile-Time Conditions

- `CTE_TERMTREES`

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

Manual review status: reviewed for porting-relevant behavior on 2026-06-22; updated for store-owned-link performance review on 2026-07-11.

Source files reviewed: `TERMS/cte_termtrees.h`, `TERMS/cte_termtrees.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `TERMS` covering 2 source file(s), about 561 lines, 9 scanned public declarations, 1 scanned internal function definitions, and 10 structured function-comment blocks.
- Functionality of term-top indexed trees (I found that I can cleanly separate this from the termbank stuff). There are two sets of funktions for the manangment of term trees in CLIB: Funktions operating only on the top cell, and functions descending the term structure. Top level functions implement a conventional AVL tree with key f_code....
- Term code. Term sharing, banks, variable conventions, type banks, and substitution/unification behavior are core semantic constraints.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `TermTopCompare` switches from a first-order type-pointer equality assertion to higher-order type-pointer ordering through the process-global `problemType`. Rust preserves that assertion boundary and activates the parsed problem dialect before proof-search term indexing, but a cleaned term-bank API should pass the problem type explicitly into top-cell comparison instead of relying on parser-global residue.
- C embeds `lson`/`rson` index links directly in each `TermCell` and uses a stack-local dummy `TermCell` to assemble both sides during top-down splaying. Rust preserves the intrusive tree shape without heap-allocating the dummy header. A safe store-owned node-arena prototype preserved the hash, comparator, rotations, root movement, and extraction behavior but regressed paired `LUSK6.lop` CPU time by 1.31 percent, so revisit store-owned links only as part of a broader stable term arena rather than as an isolated optimization.
- Because those `lson`/`rson` links belong to the tree rather than to an immutable term value, one `TermCell` can participate in only one independently mutated `TermTree`. Copying tree roots while aliasing cells lets either tree's find, insert, extract, or splay operation relink and disconnect the other tree. C avoids this in normal parsing by allocating distinct banks; a future non-intrusive index or typed single-store ownership boundary would make the invariant enforceable instead of implicit.
- `TermTopCompare` is documented as comparing `f_code.masked_properties.args_as_pointers`, but the implementation ignores properties and compares function code, higher-order type address when applicable, arity, then argument addresses through `PCmp`'s `uintptr_t` order. Correct the stale contract comment first. A later stable allocation-ID key could remove allocator/ASLR-sensitive tree shape, but current proof and output traces require preserving process-local address order until that dependency is deliberately retired.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
