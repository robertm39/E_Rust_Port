# ObjTree and ObjMap splay topology

## Status

Completed for Bead `E_Rust_Port-j76.2.23`. Rust `ObjTree` and `ObjMap` now
preserve unchanged C top-down splay topology with safe index links and owned
typed values. The vendored C checkout remains unchanged.

## Topology

The previous Rust implementations stored values in `BTreeSet`/`BTreeMap` and
kept a separate recent-root marker. That represented successful-root locality
but could not reproduce C's rotations, nearest-node root after a miss,
extraction topology, merge insertion order, or deleter traversal order.

Both containers now own arena nodes with `Option<usize>` left/right links and a
free-slot list. Their top-down splay loop is the C algorithm expressed without
raw self-referential pointers. Store, duplicate store, successful and failed
lookup, successful and failed extraction, root extraction, and null-valued map
slots all preserve C topology. `ObjTree::find` is the mutating C-shaped lookup;
`find_binary` remains the explicit non-reorganizing alternative.

[`capture_c_topology.py`](capture_c_topology.py) compiles
[`probe_obj_splay.c`](probe_obj_splay.c) against the pinned unchanged
`BASICS.a`. Retained [`reference.json`](reference.json) records the complete
tree after every operation. Permanent Rust tests assert the same shapes,
including low/high misses, failed extraction, C stack-order merge, and
left-right-root deleter order for both containers.

## Ownership boundary

Rust owns keys and values directly in arena nodes. Root changes move only an
index, so the former `Rc` root copy is unnecessary and the public containers no
longer require `Clone`. Equivalent `ObjMap` insertion retains the original key
and replaces only its value, matching C's existing-node behavior. Extraction
and deleter callbacks move the owned values out exactly once.

C's node allocator may reuse raw addresses, but comparison functions receive
stored objects or keys, not node addresses. Rust deliberately reuses safe arena
slots without exposing allocator identity. `PTreeObjMerge`'s duplicate case is
still an assertion instead of reproducing the C release-build leak path.

[`audit_obj_owners.py`](audit_obj_owners.py) finds one direct Rust storage
owner: fingerprint-index leaves hold `Option<ObjTree<T>>`; overlap-index code
also mentions that payload type in its printing surface. Immutable diagnostic
queries in the overlap and subterm indexes use `find_binary`. Generic `ObjMap`
currently has no direct Rust production instantiation, although its complete C
surface remains available and tested. The retained audit is
[`owner-audit.json`](owner-audit.json).

## Executable owner comparison

[`compare_obj_owner.py`](compare_obj_owner.py) sends `LUSK6.lop` through stdin
to the pinned C executable and the native Rust release with `--auto --silent`.
This exercises fingerprint-index leaf payloads during proof search. Retained
[`comparison.json`](comparison.json) is byte-exact: both executables produce
the same 378-byte stdout, proof marker, empty stderr, and zero exit code.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-115-obj-splay-topology\capture_c_topology.py `
  --output target\obj-splay-reference-check.json `
  --expected experiments\2026-07-18-115-obj-splay-topology\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-115-obj-splay-topology\audit_obj_owners.py `
  --output target\obj-owner-audit-check.json `
  --expected experiments\2026-07-18-115-obj-splay-topology\owner-audit.json

cargo build --locked --release --bin eprover `
  --target-dir target\default-reference

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-115-obj-splay-topology\compare_obj_owner.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\default-reference\release\eprover.exe `
  --output target\obj-owner-comparison-check.json `
  --expected experiments\2026-07-18-115-obj-splay-topology\comparison.json
```
