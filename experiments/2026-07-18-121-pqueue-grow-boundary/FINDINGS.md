# PQueue growth reachability and raw layout

## Status

Completed for Bead `E_Rust_Port-j76.2.17`. Rust preserves the unchanged C
circular-buffer behavior and exported raw growth layout without exposing C's
uninitialized-memory read hazard. The vendored C checkout remains unchanged.

## Unchanged-C layout

[`capture_c_layout.py`](capture_c_layout.py) compiles
[`probe_pqueue.c`](probe_pqueue.c) against the pinned unchanged `BASICS.a`.
Retained [`reference.json`](reference.json) records automatic store growth,
automatic bury growth, wrapped full-ring growth, direct full-ring growth, and
direct non-full growth.

The non-full probe initializes every old slot before calling `PQueueGrow`, then
reads only slots the source copy loop initialized. It proves the resulting
`size=8`, `head=2`, `tail=4`, cardinality 6 layout and copied slots without
reading the new allocation's uninitialized slots 4 and 5. Absolute-index
iteration proves those two holes are nevertheless considered live by C.

## Production boundary

[`audit_pqueue_owners.py`](audit_pqueue_owners.py) retains the complete mapping
in [`owner-audit.json`](owner-audit.json):

- No C production owner calls exported `PQueueGrow` directly. Only inline
  `pqueue_store` and `pqueue_bury` call it, after an insertion makes the ring
  full.
- No Rust production owner calls `grow_c_raw` directly. Rust's `store` and
  `bury` are likewise its only production call sites.
- SInE and the three term matching/unification owners retain `PQueue` for
  mixed tag/payload, bury, or stack-view behavior. Proof derivation, server
  sessions, and TCP channels use `VecDeque` for the C owners' pure FIFO subset.

The public raw method is retained because `PQueueGrow` is an exported C
surface and its absolute layout is observable. Rust represents backing slots
as `Option<T>`: full-ring calls remain exact, while a direct non-full call
creates `None` holes and a later raw read panics instead of invoking undefined
behavior. Permanent tests pin both the copied stale slots and the holes.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-121-pqueue-grow-boundary\capture_c_layout.py `
  --output target\pqueue-grow-reference-check.json `
  --expected experiments\2026-07-18-121-pqueue-grow-boundary\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-121-pqueue-grow-boundary\audit_pqueue_owners.py `
  --output target\pqueue-grow-owner-audit-check.json `
  --expected experiments\2026-07-18-121-pqueue-grow-boundary\owner-audit.json
```
