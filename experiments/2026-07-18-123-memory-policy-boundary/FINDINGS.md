# Memory/NewMem policy and safe ownership boundary

## Status

Completed for Bead `E_Rust_Port-j76.2.15`. Rust preserves both C allocator
policies as an owned compatibility model without installing a raw process-wide
allocator. The vendored C checkout remains unchanged.

## Exact C policies

[`capture_c_policies.py`](capture_c_policies.py) compiles
[`probe_memory_policy.c`](probe_memory_policy.c) against the pinned unchanged C
sources in both modes. Retained [`reference.json`](reference.json) proves:

- the default old allocator keeps exact-size freelists from `sizeof(MemCell)`
  through 8,191 bytes and really empties them on `MemFlushFreeList`;
- newmem rounds to 16-byte buckets through index 8,191;
- effective byte requests 1 and 255 populate 1,024-block chunks, while 256
  and 4,096 allocate one block at a time;
- 131,056 bytes remains bucketed while 131,057 bypasses the bucket array; and
- newmem's `MemFlushFreeList` leaves every retained block in place.

The 255/256 boundary is easy to misread: `MEM_CHUNKLIMIT` is spelled
`4096 / MEM_ALIGN`, but `SizeMallocReal` compares the effective byte size
directly to the resulting value 256. Rust now has a permanent regression for
both sides.

## Safe ownership boundary

[`audit_memory_owners.py`](audit_memory_owners.py) retains the complete mapping
in [`owner-audit.json`](owner-audit.json). Neither `MemoryBlock` nor
`MemoryPolicy` has a production Rust owner outside the two basics compatibility
modules. Executable code uses typed vectors, boxes, and shared handles.

Rust therefore preserves policy-visible sizes, buckets, reuse, retry/fatal
wrappers, counters, string copies, integer-array initialization, and flush
behavior without recreating raw allocator addresses. Fresh blocks are
initialized byte vectors, exact sizes travel with the owner, and frees cannot
be routed into a mismatched bucket. Allocator-address identity, debug poison
words, and true uninitialized bytes are excluded intentionally; stable
owner-specific ordering is used where C accidentally exposes addresses.

The compatibility newmem chunk model allocates safe owned blocks rather than
one raw contiguous slab. Because it has no production owner, this difference
does not affect executable throughput; a whole-prover allocator replacement
would need independent profiling and proof-order analysis.

## Reproduction

```powershell
C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-123-memory-policy-boundary\capture_c_policies.py `
  --output target\memory-policy-reference-check.json `
  --expected experiments\2026-07-18-123-memory-policy-boundary\reference.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-123-memory-policy-boundary\audit_memory_owners.py `
  --output target\memory-owner-audit-check.json `
  --expected experiments\2026-07-18-123-memory-policy-boundary\owner-audit.json
```
