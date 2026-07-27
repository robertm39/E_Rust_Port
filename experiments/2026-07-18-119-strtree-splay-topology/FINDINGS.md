# StrTree splay topology and owned C-string keys

## Status

Completed for Bead `E_Rust_Port-j76.2.19`. Rust `StrTree` now preserves the
unchanged C top-down splay topology and C-string key behavior through safe
index-linked owned nodes. The vendored C checkout remains unchanged.

## Representation and operations

The previous Rust implementation used `BTreeMap` plus a recent-root marker.
That preserved sorted contents but not the root and child topology produced by
C after duplicate insertion, failed lookup, failed extraction, or deletion.

Rust now stores owned keys and two owned values in an arena with `Option<usize>`
left/right links and safe free-slot reuse. `find`, `find_mut`, insertion,
extraction, and deletion use the C-shaped splay loop; misses leave the nearest
node at the root. `find_binary` is an explicit non-reorganizing query.
Extraction transfers the stored `String` and values without cloning. In-order
traversal remains sorted.

[`capture_c_topology.py`](capture_c_topology.py) compiles
[`probe_strtree.c`](probe_strtree.c) against the pinned unchanged `BASICS.a`.
The probe is compiled with `NDEBUG` to match that optimized archive's inline
memory-cell layout. Retained [`reference.json`](reference.json) records every
tree after insertion, duplicate insertion, hit/miss lookup, and hit/miss
extraction, plus traversal, copied-key ownership, embedded-NUL termination, and
non-ASCII byte order. Permanent Rust tests assert the same topology and
outcomes.

## C-string boundary

C `StrTreeStore` calls `SecureStrdup`, so the tree owns a copy terminated at the
first NUL. `strcmp` orders the non-NUL prefix as unsigned bytes. Rust mirrors
this by truncating an owned key at its first embedded NUL, ignoring query
suffixes after NUL, and comparing UTF-8 bytes rather than locale or Unicode
collation. Duplicate insertion preserves the first stored key and values.

The safe generic Rust API accepts `&str`; it therefore excludes arbitrary
invalid-UTF-8 byte sequences that a raw C `char*` could contain. The four direct
Rust owners already operate on scanner, include-path, batch-selector, and
example-name strings, so this restriction does not narrow their executable
behavior. If a future typed owner needs opaque bytes, it should receive an
owner-specific byte boundary rather than weakening this API.

## Owner boundary

[`audit_strtree_owners.py`](audit_strtree_owners.py) retains the safe
representation/API checks, four direct generic Rust owner modules, and 28
unchanged-C owner files in [`owner-audit.json`](owner-audit.json). Mutable
selector paths keep C-shaped splaying. Shared-reference include-skip and example
name queries use the explicit non-reorganizing binary query instead of hiding
mutation behind interior mutability. Full all-feature tests exercise these
owners together with the standalone topology regression.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-119-strtree-splay-topology\capture_c_topology.py `
  --output target\strtree-reference-check.json `
  --expected experiments\2026-07-18-119-strtree-splay-topology\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-119-strtree-splay-topology\audit_strtree_owners.py `
  --output target\strtree-owner-audit-check.json `
  --expected experiments\2026-07-18-119-strtree-splay-topology\owner-audit.json
```
