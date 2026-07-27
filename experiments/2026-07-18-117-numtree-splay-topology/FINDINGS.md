# NumTree splay topology

## Status

Completed for Bead `E_Rust_Port-j76.2.21`. Rust `NumTree` now preserves the
unchanged C top-down splay topology through safe index-linked owned nodes. The
vendored C checkout remains unchanged.

## Representation and operations

The previous Rust implementation used `BTreeMap` plus a recent-root marker.
That preserved sorted contents but not the root and child topology produced by
C after duplicate insertion, failed lookup, or failed extraction. It also
rendered debug output as a sorted list even though C prints the current tree in
preorder.

Rust now stores the two owned values in an arena with `Option<usize>` left/right
links and safe free-slot reuse. `find`, `find_mut`, insertion, and extraction
use the C-shaped splay loop; misses leave the nearest node at the root.
`find_binary` and `max_node` are explicit non-reorganizing queries. Root
extraction moves the owned entry without cloning. In-order traversal remains
sorted, and limited traversal initializes the path to the first key greater
than or equal to the limit in logarithmic tree descent rather than filtering a
full traversal.

[`capture_c_topology.py`](capture_c_topology.py) compiles
[`probe_numtree.c`](probe_numtree.c) against the pinned unchanged `BASICS.a`.
The probe is compiled with `NDEBUG` to match that optimized archive's inline
memory-cell layout; mixing the debug inline `PStackFree` with the optimized
archive is not ABI-compatible. Retained [`reference.json`](reference.json)
records every tree after insertion, duplicate insertion, hit/miss lookup,
non-reorganizing max lookup, hit/miss/root extraction, plus node count,
full/exact/inexact/out-of-range limited traversals, and keys-only debug output.
Permanent Rust tests assert the same topology and orders.

## Debug and integer boundaries

C `NumTreeDebugPrint` prints the current topology in preorder. It prints an
explicit empty child only when that child's parent has at least one child and
uses four visible spaces per tree level. Rust now preserves that layout. The
non-keys-only mode also preserves the value and child-pointer lines using
implementation-native Rust node addresses. Exact `%p` text is inherently
allocator- and platform-dependent, so only the keys-only layout is retained as
byte-exact C evidence.

The Rust key is `i64`, matching the pinned LP64 Linux C `long`. C compares by
signed subtraction, which is undefined on overflow; Rust uses total integer
comparison and therefore remains defined at extreme keys.

## Owner boundary

[`audit_numtree_owners.py`](audit_numtree_owners.py) retains the safe
representation, API checks, five direct generic Rust owner modules, and the 45
unchanged-C owner files in [`owner-audit.json`](owner-audit.json). Mutable owner
paths keep C-shaped splaying. Public Rust queries that only have shared access,
such as annotation/example lookup, use the explicit non-reorganizing binary
query rather than hiding mutation behind interior mutability. Full all-feature
tests exercise these owner paths together with the exact standalone topology
regression.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-117-numtree-splay-topology\capture_c_topology.py `
  --output target\numtree-reference-check.json `
  --expected experiments\2026-07-18-117-numtree-splay-topology\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-117-numtree-splay-topology\audit_numtree_owners.py `
  --output target\numtree-owner-audit-check.json `
  --expected experiments\2026-07-18-117-numtree-splay-topology\owner-audit.json
```
