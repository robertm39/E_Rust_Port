# Detailed BASICS reconciliation

## Status

Accepted for the 39 remaining open `basics` records under Beads
`E_Rust_Port-j76.4`. Direct review found no missing production BASICS behavior.
The records describe intentional assertion contracts, safe replacements for C
undefined behavior or uninitialized storage, narrow compatibility adapters, or
already reconciled owner/topology/platform boundaries. No Rust or C source
changed.

## Review decisions

| Record | Decision |
|---|---|
| 2 | Keep negative `DDArray` access panic-shaped in the C-compatible API. Any optional Rust lookup belongs in a separately named wrapper. |
| 12 | Keep empty and out-of-range `DStack` operations panic-shaped. This is the tested C assertion contract. |
| 13 | Retain reference-count and final-release compatibility helpers, while ordinary Rust ownership remains value/handle based and cannot expose an invalid descriptor after release. |
| 14 | Continue accepting only `usize` in safe `DStr::address`. A negative C index is an out-of-buffer pointer with no valid drop-in result and no production caller. |
| 15 | Keep descriptor append restricted to distinct borrows. No reachable C owner requires reallocating self-append alias behavior, so an unsafe compatibility operation would add hazard without behavior. |
| 24 | Preserve the C resource footer: self plus child user/system time and parent-only raw `ru_maxrss` under the legacy pages label. Target-specific resident units remain explicit evidence fields. |
| 25 | Preserve `ELog`'s PID file, CPU prefix, append behavior, and trailing-newline-to-stderr split, while returning I/O failures instead of dereferencing a null `FILE*`. |
| 27 | Keep `FixedDArray` safely zero-initialized. No owner reads allocation residue, and an internal uninitialized builder would require a separate fill-before-read proof. |
| 28 | Retain assertion-shaped vector size/index invariants for component operations; checked Rust-only constructors can remain separate. |
| 36 | Let Rust ownership replace `FREE`/`SizeFree` debug nulling and poison writes. Those non-semantic post-free side effects are neither safely observable nor needed by production owners. |
| 37 | Keep explicit old exact-size and new aligned-chunk allocator policies. System-malloc-only mode has no Rust production owner and would be a separate compatibility policy if ever needed. |
| 41 | Keep the C-shaped nonempty heap pop fatal/panic-shaped while retaining the distinct optional draining method. |
| 42 | Preserve the historically confusing `decr_key`/`incr_key` directions in compatibility methods; clearer names may only be additive. |
| 44 | Keep signed-index heap wrappers only at the C boundary. Native callers use `usize` and optional lookup rather than signed sentinels. |
| 51 | Preserve `NumTree` limited traversal at the first key greater than or equal to the limit. The source comment is wrong; exact C topology probes pin the implementation. |
| 52 | Preserve the same greater-than-or-equal limited traversal rule for `NumXTree`; no caller relies on the stale prose. |
| 56 | Keep typed owned object-tree payloads, explicit splayed versus binary lookup, and one-time deletion ownership. Raw `void*` owner ambiguity is not a compatibility requirement. |
| 60 | Keep Linux `getrusage` as the primary resource owner with `/proc` fallback and explicit platform units. |
| 61 | Keep Linux process CPU time on C `clock()` semantics; unsupported targets retain a documented monotonic fallback rather than pretending exactness. |
| 63 | Retain the now-exact single overwriteable start slot per named performance counter, with the exact 13-name/order output and saturation owner. |
| 64 | Preserve the five-entry `POCompareSymbol` table in discriminant order through a safe typed constant/method surface. |
| 66 | Preserve sign-based quasi-to-partial conversion for raw compatibility inputs; new ordering code should use typed results. |
| 67 | Keep negative `PDArray` access panic-shaped like C assertions. |
| 69 | Keep raw clear non-growing for covered slots but panic on uncovered slots rather than performing C's out-of-allocation write. Ordinary code uses checked deletion. |
| 72 | Preserve mutating grow/shift reads for `PDRangeArr` compatibility, including `IntMap`; side-effect-free Rust lookups remain distinct. |
| 75 | Keep returned `Arc<str>` values valid after registry clearing. Reproducing dangling C pointers has no legitimate consumer. |
| 76 | Keep the mutex-protected ordered registry rather than C splay locality/address reuse. Production callers need lifetime extension, not pointer identity. |
| 79 | Retain explicit anchor validation and panic-shaped raw list contracts; checked optional traversal stays separate. |
| 83 | Keep the generic 64-slot C-shaped local stack and safe two-slot portable tagged representation. The only hot C tagged owner uses typed Rust frames, and retained profiles do not justify raw low-bit handle packing. |
| 85 | Keep empty queue extract/look panic-shaped, with optional behavior confined to higher-level queue APIs. |
| 87 | Quarantine exported raw queue growth. Full-ring behavior is exact; a direct non-full call creates safe `None` holes and raw reads panic instead of reading uninitialized C memory. |
| 88 | Preserve both masked-return names for compatibility while keeping the Rust Boolean predicate visibly distinct from mask extraction. |
| 89 | Keep assertion-shaped and `try_` stack operations as separate APIs so C contracts and optional draining cannot be confused. |
| 90 | Retain only the audited `"%4ld."` integer and `%p` pointer renderers required by reachable callers. A generic printf interpreter would add unsupported syntax and risk. |
| 93 | Preserve root-right-left `PTree` stack/debug traversal separately from sorted traversal. Live `Rc` allocation identity reproduces the direct pointer-tree owners without making pointer order semantic elsewhere. |
| 95 | Keep exact top-down quadtree splaying on hits and misses, including nearest-boundary miss roots. The earlier partial-locality concern is now fully resolved and owner-tested. |
| 97 | Retain the process-global registered-memory compatibility owner and panic/`try_` wrappers, while the sole production scratch consumer uses typed thread-local storage. |
| 100 | Keep registered byte buffers zero-initialized, including newly grown tails. No production consumer requires allocation residue, and the typed hot owner preserves C growth/reuse behavior. |
| 107 | Preserve global verbosity thresholds, program-name formatting, stderr routing, and flushes for compatibility; explicit per-run writers remain preferred for new Rust owners. |

## Evidence

The low-level regressions cover every decision:

- dynamic/fixed arrays, stacks, lists, heaps, and queues pin assertion,
  growth, signed-index, raw-layout, and checked-wrapper boundaries;
- dynamic strings pin C reference counts, final release, C-string views,
  address-at-NUL, and all reachable append behavior;
- resource/footer and ELog tests pin stream ownership, platform measurement,
  CPU clocks, and failure handling;
- memory and registered-scratch studies prove old/new allocator policies,
  zeroed safe storage, typed hot ownership, and exact executable behavior;
- numeric, object, pointer, and quad tree studies capture unchanged-C splay
  topology, limited traversal, miss roots, traversal order, identity policy,
  deleter order, and production owners;
- partial-order/property tests pin discriminants, table order, raw sign
  conversion, and masked results; and
- permanent strings, local tagged stacks, performance counters, and verbosity
  retain exact observable surfaces while excluding dangling pointers,
  uninitialized reads, and unsafe handle packing.

The latest exact candidate passes 4,429 tests, all 50 main-prover cases, and all
216 support-tool cases with zero unexpected differences.

## Audit

[`audit_basics_reconciliation.py`](audit_basics_reconciliation.py) pins the
exact 39 migrated identities and content hashes, checks thirteen grouped
source/implementation/evidence contracts, and digests the 47 unchanged C
units, 25 Rust owners, status ledger, twelve retained owner findings, and
current validation reference. The audit is independent of issue status, so it
remains reproducible after closure.

## Validation

The source audit, Python syntax check, C-source documentation coverage,
Change Later wording, local links, manual-regeneration preservation, and
`git diff --check` pass. The unchanged implementation is covered by the exact
Experiment 046 lifecycle:

- Rustfmt and strict all-target/all-feature pedantic Clippy pass;
- 4,418 library plus 11 integration tests pass, 4,429 total;
- native release and compile-only Windows GNU x64 all-target/all-feature
  builds pass; and
- 50 main plus 216 support-tool comparisons have zero unexpected differences.

No Rust or C toolchain ran on the local Windows host. The vendored C checkout
is clean.

Reproduce the source audit locally:

```powershell
.\.venv\Scripts\python.exe experiments/2026-07-25-050-basics-reconciliation/audit_basics_reconciliation.py `
  --repo . `
  --expected experiments/2026-07-25-050-basics-reconciliation/audit-reference.json
```
