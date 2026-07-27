# Experiment 263: Size-class lock sentinels

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can each exact-size list embed its own lock state in the head pointer, replacing
the accepted process-wide spin lock plus separate head load/store traffic with
one atomic exchange and one release store?

## Setup

- Parent source: commit `e53a9fb7` (`perf: cache exact-size allocations`),
  accepted Experiment 261.
- Parent native executable:
  `target/native-261-global-size-freelist/release/eprover.exe`.
- Candidate native executable:
  `target/native-263-size-class-lock/release/eprover.exe`.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.
- Timing protocol: four alternating warmup pairs followed by 64 alternating
  measured pairs.

## Candidate

Every cacheable block is 16-byte aligned, so address one cannot be a real list
head. The candidate used that impossible unaligned pointer as a locked sentinel
in each `AtomicPtr`:

1. `swap(sentinel, Acquire)` took both the size-class lock and its current
   head in one operation;
2. a sentinel result meant another thread owned that class and caused a spin;
3. the guard's `Drop` published the updated head with a release store.

Allocation, deallocation, and failure flushing retained the accepted exact-size
and normalized-layout invariants. Per-class locking remained safe across
threaded server/control modes, reduced unrelated size-class contention, and
avoided lock-free-stack ABA hazards.

## Validation before timing

- The candidate proves LUSK6 and exits zero in a direct run.
- All four focused allocator tests pass with all features, including parallel
  reuse.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate executable is 8,903,168 bytes, 25,600 bytes smaller than the
  8,928,768-byte parent.

As in Experiment 262, WSL reported that no distributions were installed and
neither Docker nor Podman was available. Native production timing was used as
the first falsification gate; the negative result made restoring the profiling
runtime and running compatibility matrices unnecessary.

## Native result

All 128 measured processes prove and exit zero. Across all 64 pairs, the
candidate is worse:

- wall mean regresses 0.751963%, from 1.406516 to 1.417092 seconds;
- process-CPU mean regresses 0.927743%, from 1.368408 to 1.381104 seconds;
- wall median regresses 0.582585%, while CPU median is equal;
- mean paired wall and CPU changes regress 0.794829% and 0.993739%;
- median paired wall and CPU changes regress 0.607530% and 0.574713%;
- the candidate wins only 25 wall pairs and 22 CPU pairs, with ten CPU ties.

The stable last 32 pairs are more decisive:

- wall and CPU means regress 0.549427% and 1.273654%;
- wall median regresses 0.181265%, while CPU median is equal;
- mean paired wall and CPU changes regress 0.619877% and 1.345144%;
- median paired wall and CPU changes regress 0.495312% and 0.574713%;
- the candidate wins 13 wall pairs and only nine CPU pairs, with seven CPU
  ties.

The measured rows are retained in `native-lusk.csv`; warmup rows are under the
ignored artifact directory.

## Decision

Reject. The smaller per-class locking implementation is safe and shrinks the
binary, but its atomic exchange path is slower than the accepted global
compare-exchange lock in production timing. Restore Experiment 261 source
byte-for-byte. The accepted baseline remains 9,106,424,013 Callgrind
instructions, or 1.733117 times C.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-261-global-size-freelist\release\eprover.exe `
  -CandidateExe .\target\native-263-size-class-lock\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-025-size-class-lock\native-lusk.csv
```
