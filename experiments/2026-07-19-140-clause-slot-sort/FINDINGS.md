# Clause-slot permutation sort

## Question

Can sparse clause sets be compacted and sorted without materializing every
160-byte Rust `Clause` in one contiguous temporary, while preserving proof
order and keeping performance comparable?

## Setup

- Parent source includes experiment 139's bounded diversity-variable scratch.
- C reference: archived FOL build from upstream commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.
- Windows production candidate:
  `target/diversity-scratch/release/eprover.exe`, SHA-256
  `8405250E488161AA3B03F1DB054419659F341E95775E5A81EE6777700111B342`.
- Deterministic fixture: unchanged `LUSK6.lop` under WSL Callgrind with
  `--auto --silent --cpu-limit=600 --memory-limit=2048 --detsort-rw
  --detsort-new`.

SWV851 reached 1,258,291 clauses near its resource boundary. The old
`SparseClauseStore::sort_unstable_by` moved all clauses into a `Vec<Clause>`;
its roughly 192 MiB request matched the observed allocator failure. Raising
the Windows Job Object allowance only postponed the same ownership problem.

## Source comparison

C `ClauseSetSort()` extracts clause pointers into a `PStack`, calls
`PStackSort()`, and reinserts the pointed-to clauses. Its temporary therefore
stores one machine word per clause, not one complete clause.

Rust now compacts occupied sparse slots forward in place. Sorting allocates a
single `Vec<usize>`, orders those indices through the existing clause
comparator, converts the destination-to-source result to its inverse inside
the same vector, and applies the permutation with in-place clause swaps.
Sparse compaction uses the same page storage directly and truncates trailing
pages instead of collecting a second complete clause vector.

## Deterministic result

The candidate profile contains 17,264,035,215 instructions. This is 31,538,030
instructions, or 0.18%, above experiment 139's 17,232,497,185 profile. The
exact proof and all 4,873 processed-clause calls are unchanged. The small cost
is the expected comparator indirection and permutation application; total
instructions remain 13.25% below the retained 19,899,749,157 baseline.

The retained profile is
`.artifacts/experiments/2026-07-19-140-clause-slot-sort/callgrind-current.out`.
The focused C/Rust LUSK6 report at
`.artifacts/e-compare/20260719-142718-489707/` is exact.

## Resource results

The change removes the identified 192 MiB sort request, but it does not by
itself close the port's allocator-versus-cooperative-time-limit boundary:

- the 180-second BOO/SWV stress report at
  `.artifacts/e-compare/20260719-142736-847420/` reaches much smaller Rust
  allocation failures (320 KiB and 168 bytes) after about a minute, while C
  reaches its CPU limit at 180 seconds;
- the standard 50-case report at
  `.artifacts/e-compare/20260719-144224-592472/` keeps BOO exact
  `ResourceOut`/8, all proof cases exact, and the synthetic 16 MiB case exact;
  SWV instead reaches an 8-byte allocation failure immediately before its
  cooperative CPU check;
- the isolated standard-limit SWV rerun at
  `.artifacts/e-compare/20260719-145336-503287/` reproduces the residual
  168-byte failure.

The standard matrix therefore has two unexpected differences: residual SWV
resource handling and the already-open synthetic one-second LUSK6 throughput
gap. The declared `sledgehammer.p` proof-text difference remains expected.
GEO288, HEN011, LUSK6, and `LUSK6ext` retain exact output at 11.26, 55.60,
3.16, and 8.60 seconds respectively.

This evidence separates two owners: whole-clause sort memory is fixed, while
Windows allocation failure and asynchronous CPU-limit behavior remain open.
No Job Object allowance change is included.

## Falsification checks

- A regression inserts more than one sparse-store page, removes holes, sorts
  across the page boundary, and checks order, compaction metadata, and bounded
  page capacities.
- The existing clause-set suite and the new cross-page regression pass,
  including evaluated-object and index rebuilding across removal, sorting,
  and compaction.
- The all-target/all-feature check and focused LUSK6 comparison pass.
- Callgrind pins the exact proof and clause count and quantifies the 0.18%
  instruction tradeoff.
- The standard matrix covers FOL, HO, proof documentation, parser modes,
  resource limits, stdin, and malformed input.
- The vendored C checkout remains unchanged.

## Decision

Accept the pointer-shaped slot permutation and allocation-free compaction. It
eliminates a confirmed 192 MiB temporary and follows C's one-word-per-clause
sort shape at a measured 0.18% instruction cost. Do not claim resource parity:
continue with a dedicated allocator/time-limit owner for the remaining tiny
allocation failures, and keep the main acceptance issue open.
