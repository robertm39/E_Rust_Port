# FloatTree splay topology and NaN boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.20`. Rust `FloatTree` now preserves the
unchanged C top-down splay topology and its exact signed-zero, infinity, and NaN
behavior through safe index-linked owned nodes. The vendored C checkout remains
unchanged.

## Representation and ordered operations

The previous Rust implementation used `BTreeMap` with a lawful total-order
wrapper plus a recent-root marker. That preserved sorted contents but not the
root and child topology produced by C after duplicate insertion, failed lookup,
or failed extraction.

Rust now stores the two owned values in an arena with `Option<usize>` left/right
links and safe free-slot reuse. `find`, `find_mut`, insertion, and extraction
use the C-shaped splay loop; ordered misses leave the nearest node at the root.
`find_binary` is an explicit non-reorganizing query. Extraction moves the owned
entry without cloning, and in-order traversal remains sorted for ordered keys.

[`capture_c_topology.py`](capture_c_topology.py) compiles
[`probe_floattree.c`](probe_floattree.c) against the pinned unchanged
`BASICS.a`. The probe is compiled with `NDEBUG` to match that optimized
archive's inline memory-cell layout. Retained [`reference.json`](reference.json)
records every ordered tree after insertion, duplicate insertion, hit/miss
lookup, and hit/miss extraction, plus node count, traversal, signed-zero bits,
and the complete NaN state matrix. Permanent Rust tests assert the same
topology and outcomes.

## Signed zero, infinity, and NaN

C uses subtraction and `< 0`/`> 0` tests for structural direction. Rust uses
`partial_cmp` for the same defined ordering without manufacturing a total float
order. Equal infinities and signed zeros stop the splay like C. Duplicate
`-0.0`/`+0.0` insertion preserves the already-stored node's exact zero bits,
and lookup/extraction by the opposite zero succeeds.

When either structural operand is NaN, C subtraction is unordered, so both
direction tests are false and the splay stops at the current root. Insertion
then treats that unordered result like a duplicate, while lookup and extraction
use IEEE equality and cannot match NaN. Therefore:

- inserting NaN into a nonempty tree rejects it as a duplicate of the current
  root without changing that root or its values;
- inserting NaN into an empty tree succeeds, but the retained NaN node cannot be
  found, extracted, or deleted by key; and
- that NaN root causes every later numeric or NaN insertion to be rejected as a
  duplicate.

Rust preserves this accidental boundary exactly. A future production owner
should reject NaN before entering the compatibility container if that behavior
is undesirable.

## Owner boundary

[`audit_floattree_owners.py`](audit_floattree_owners.py) retains the safe
representation/API checks and proves in [`owner-audit.json`](owner-audit.json)
that neither unchanged C nor Rust has a direct production owner outside the
compatibility module. The exact standalone topology and exceptional-value probe
is therefore the relevant behavioral evidence; no executable owner comparison
is claimed.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-118-floattree-splay-topology\capture_c_topology.py `
  --output target\floattree-reference-check.json `
  --expected experiments\2026-07-18-118-floattree-splay-topology\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-118-floattree-splay-topology\audit_floattree_owners.py `
  --output target\floattree-owner-audit-check.json `
  --expected experiments\2026-07-18-118-floattree-splay-topology\owner-audit.json
```
