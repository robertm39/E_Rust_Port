# IntMap compatibility and production-owner boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.14`. Rust retains the observable C
representation and accounting rules while keeping read-only Rust iteration
side-effect-free. The vendored C checkout remains unchanged.

## Exact unchanged-C behavior

[`capture_c_intmap.py`](capture_c_intmap.py) compiles
[`probe_intmap.c`](probe_intmap.c) against the pinned C archive with
`CONSTANT_MEM_ESTIMATE`, the compatibility mode used by Rust proof-state
storage accounting. Retained [`reference.json`](reference.json) proves:

- single, dense-array, and sparse-tree shapes retain 20-, 76-, and 68-byte
  estimates;
- inserting keys 0 then 100 takes the `IMSingle` argument-order bug and creates
  a 104-slot array with a 460-byte estimate, while inserting the same keys in
  reverse creates the 68-byte tree;
- repeated references to the same null array slot raise `entry_no` from 2 to 3
  and then 4 without creating a value;
- failed array lookup and deletion below the backing offset grow it from
  offset 10/size 8 to offset 2/size 16 while leaving logical bounds and entry
  count unchanged; and
- an array iterator starting at that same raw lower key performs the same
  growth before returning key 10.

The insertion-order result is not merely an internal container choice. C
`IntMapStorage` changes by 392 bytes for the same logical entries, and FV-index
and PDT accounting observe that difference.

## Production-owner decision

[`audit_intmap_owners.py`](audit_intmap_owners.py) retains the complete mapping
in [`owner-audit.json`](owner-audit.json). The only production Rust owners are:

- `FvIndex.successor_storage`, a parallel compatibility map used for signed
  insertion-side storage deltas; and
- `PdNode.fun_alternatives`, used for PDT constant-memory accounting while
  inserting guarded slots and deleting known child keys.

Neither owner calls `get_val` or `iter_range_c_mut`. The miss-triggered growth
and raw-lower iterator behavior therefore remain isolated in explicit
compatibility methods. Ordinary `iter_range` uses checked existing slots and
does not allocate. PDT sets a newly requested slot immediately, so repeated
null-reference inflation is not a production path; it remains represented
because it affects the public C-shaped API and density decisions.

Replacing the asymmetric transition inside the compatibility map would change
storage counters used by proof-state cleanup and index statistics. Any future
logical-map cleanup should instead be a separate typed container or wait until
those compatibility counters are retired with end-to-end proof-order evidence.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-124-intmap-compatibility-boundary\capture_c_intmap.py `
  --output target\intmap-c-reference-check.json `
  --expected experiments\2026-07-18-124-intmap-compatibility-boundary\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-124-intmap-compatibility-boundary\audit_intmap_owners.py `
  --output target\intmap-owner-audit-check.json `
  --expected experiments\2026-07-18-124-intmap-compatibility-boundary\owner-audit.json
```
