# NumXTree splay topology

## Status

Completed for Bead `E_Rust_Port-j76.2.22`. Rust `NumXTree` now preserves the
unchanged C top-down splay topology through safe index-linked owned nodes. The
vendored C checkout remains unchanged.

## Representation and operations

The previous Rust implementation used `BTreeMap` plus a recent-root marker.
That preserved sorted contents but not the root and child topology produced by
C after duplicate insertion, failed lookup, or failed extraction.

Rust now stores four-value entries in an arena with `Option<usize>` left/right
links and safe free-slot reuse. `find`, `find_mut`, insertion, and extraction
use the C-shaped splay loop; misses leave the nearest node at the root.
`find_binary` and `max_node` are explicit non-reorganizing queries. Root
extraction moves the owned entry without cloning. In-order traversal remains
sorted, and limited traversal initializes the path to the first key greater
than or equal to the limit in logarithmic tree descent rather than filtering a
full traversal.

[`capture_c_topology.py`](capture_c_topology.py) compiles
[`probe_numxtree.c`](probe_numxtree.c) against the pinned unchanged
`BASICS.a`. The probe is compiled with `NDEBUG` to match that optimized
archive's inline memory-cell layout; mixing the debug inline `PStackFree` with
the optimized archive is not ABI-compatible. Retained
[`reference.json`](reference.json) records every tree after insertion,
duplicate insertion, hit/miss lookup, non-reorganizing max lookup,
hit/miss/root extraction, plus node count and full/exact/inexact/out-of-range
limited traversals. Permanent Rust tests assert the same topology and orders.

## Value and integer boundaries

The C header promises four `IntOrP` slots and says `NumXTreeStore` zeroes slots
beyond the first two, but that function allocates a raw cell and initializes
only slots zero and one. There are no C production callers of `NumXTreeStore`;
the formula-definition owners allocate cells directly and initialize the later
slots before reading them. Rust retains deterministic `Default` values for the
two tail slots, honoring the public contract without exposing uninitialized
memory. `insert_entry` remains available when all four values are supplied.

The Rust key is `i64`, matching the pinned LP64 Linux C `long`. C compares by
signed subtraction, which is undefined on overflow; Rust uses total integer
comparison and therefore remains defined at extreme keys.

## Owner boundary

[`audit_numxtree_owners.py`](audit_numxtree_owners.py) retains the safe
representation and API checks in [`owner-audit.json`](owner-audit.json). It
finds no direct generic `NumXTree` instantiation outside its Rust compatibility
module. The unchanged C owner inventory is retained for formula-definition and
higher-order inference code, whose Rust ports use owner-specific typed state.
Because there is no direct Rust production owner to exercise, this slice uses
the exact C topology probe rather than claiming an executable owner comparison.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-116-numxtree-splay-topology\capture_c_topology.py `
  --output target\numxtree-reference-check.json `
  --expected experiments\2026-07-18-116-numxtree-splay-topology\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-116-numxtree-splay-topology\audit_numxtree_owners.py `
  --output target\numxtree-owner-audit-check.json `
  --expected experiments\2026-07-18-116-numxtree-splay-topology\owner-audit.json
```
