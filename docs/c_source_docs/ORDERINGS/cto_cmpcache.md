<!-- BEGIN AUTO-GENERATED: c_source_docs -->
# ORDERINGS / cto_cmpcache

## Source Files

- [ORDERINGS/cto_cmpcache.h](../../../eprover/ORDERINGS/cto_cmpcache.h)
- [ORDERINGS/cto_cmpcache.c](../../../eprover/ORDERINGS/cto_cmpcache.c)

## Purpose

Cache structure for the local caching of ordering results for LPO (and potentially RPO and other mainly recursive orderings). the GNU Lesser General Public License. <1> Sat Dec 25 00:50:42 MET 1999

Within the source tree, this unit belongs to `ORDERINGS`. Term ordering implementations and support structures, including KBO, LPO, order-control blocks, precedence/weight handling, and comparison caching.

Authors noted in source headers: Stephan Schulz

## Public Surface

Exported declarations are primarily taken from headers. For standalone program sources, externally visible definitions are listed as the source scan finds them.

### Types

- `CmpCache_p`

### Macros And Constants

- `CTO_CMPCACHE`
- `CmpCacheClear(cache)`
- `CmpCacheInit(cache)`

### Globals

- None found in the source scan.

### Exported Functions

- `CompareResult CmpCacheFind(CmpCache_p *cache, Term_p t1, DerefType d1, Term_p t2, DerefType d2)`
- `bool CmpCacheInsert(CmpCache_p *cache, Term_p t1, DerefType d1, Term_p t2, DerefType d2, CompareResult insert)`

## Implementation Notes

### Internal Functions

- `prepare_key`

### Source-Level Behavior

- `prepare_key`: Turn the 4 values into an ordered key. Return false if values are reordered, true otherwise.
- `CmpCacheFind`: Find a certain comparison in the cache.
- `CmpCacheInsert`: Insert a comparison into an LPO cache. Return false if value already existed, true otherwise.

### Dependencies

- `"cto_cmpcache.h"`
- `<clb_partial_orderings.h>`
- `<clb_quadtrees.h>`
- `<cte_termbanks.h>`

### Compile-Time Conditions

- `CTO_CMPCACHE`

## Porting Notes

- Keep the Rust port close to the C ownership model visible in this unit's allocation/free helpers and exported APIs.
- Assertions encode local invariants; translate them into debug assertions or explicit checks where callers can violate them.
- Audit global state carefully; many E modules rely on process-wide counters, caches, or option variables.
- Term sharing and term-bank insertion are semantic constraints, not just memory optimizations.
<!-- END AUTO-GENERATED: c_source_docs -->

<!-- BEGIN MANUAL REVIEW: c_source_docs -->
## Manual Review

Manual review status: reviewed for porting-relevant behavior on 2026-06-22.

Source files reviewed: `ORDERINGS/cto_cmpcache.h`, `ORDERINGS/cto_cmpcache.c`.

### Review Notes

- Reviewed as a paired implementation/header unit in `ORDERINGS` covering 2 source file(s), about 292 lines, 3 scanned public declarations, 1 scanned internal function definitions, and 3 structured function-comment blocks.
- Comparison cache; preserve invalidation and key identity assumptions if ported.
- Ordering code. Comparison outcomes, caching, precedence, and weight handling must match the C implementation because they drive simplification and inference eligibility.
- Term/type sharing affects equality and performance; do not replace pointer identity with structural equality without auditing callers.
- Ordering comparisons feed simplification and inference eligibility; preserve tie-breakers, cache use, and incomparability results.
- Cache keys are term-pointer identities plus deref counters, canonicalized so reverse lookups use inverse comparison results. Rust preserves identity keys, but exact raw-address ordering and splay-tree recent-access behavior are cleanup/performance questions to revisit with benchmarks.
- Compile-time branches are real behavior variants; decide whether each becomes a Cargo feature, cfg flag, or a single supported path.
- Assertions document invariants expected by internal callers; translate important ones into debug assertions or explicit validation.
- Global variables are often configuration or shared caches; preserve initialization and mutation timing.

### Change Later

- `CmpCache` canonicalizes cache keys with raw term pointer ordering and stores them in a splay-tree-backed `QuadTree`, so cache hit locality and even canonical key order depend on allocator addresses. Rust preserves term-identity keys with stable term ids; after drop-in compatibility, benchmark recursive LPO/LPO4 workloads before deciding whether address-like ordering or splay locality should be modeled more exactly or replaced with a cleaner deterministic cache.
- The cache stores dereference modes as part of the key but depends on callers to clear it when term-bank or substitution state can invalidate comparisons. Keep that invalidation responsibility explicit; a later ordering API should tie cache lifetime to the comparison context rather than exposing a mutable global-ish cache handle.

### Porting Focus

- Keep the generated public-surface inventory above in sync with the source, but treat this manual section as the place for compatibility judgments.
- Before replacing C idioms with safer Rust abstractions, identify whether callers depend on object identity, global state, allocation reuse, or fatal-error behavior.
- If behavior is unclear, prefer matching the C source first and adding Rust-side tests around the observed C behavior.
<!-- END MANUAL REVIEW: c_source_docs -->
