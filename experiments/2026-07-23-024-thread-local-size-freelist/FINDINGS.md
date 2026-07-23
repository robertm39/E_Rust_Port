# Experiment 262: Thread-local exact-size free lists

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can per-thread exact-size lists remove the accepted allocator's process-wide
spin lock from the single-threaded proof-search hot path while preserving safe
allocation in the port's threaded server and control modes?

## Setup

- Parent source: commit `e53a9fb7` (`perf: cache exact-size allocations`),
  accepted Experiment 261.
- Parent native executable:
  `target/native-261-global-size-freelist/release/eprover.exe`.
- Candidate native executable:
  `target/native-262-thread-local-size-freelist/release/eprover.exe`.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.
- Timing protocol: four saved alternating warmup pairs followed by 64
  alternating measured pairs.

The accepted profile attributes 135,382,198 exclusive instructions to the
global allocation function before counting the lock operations inlined into
deallocation callers. That made removing uncontended process-wide locking a
plausible bounded follow-up.

## Candidate

The candidate replaced the single locked list array with a const-initialized
thread-local array:

- each thread owned all exact-size list links that it accessed;
- blocks remained transferable across threads because a deallocation joined
  the receiving thread's matching size class;
- a thread-local destructor returned all cached blocks directly to `System`;
- allocation and deallocation after TLS destruction fell back directly to
  `System` with the normalized 16-byte layout;
- an allocation failure flushed the calling thread's cache before retry.

The unsafe boundary remained in the existing private allocator module.
Thread-local ownership excluded concurrent list access, and no operation
re-entered the global allocator while the `UnsafeCell` was mutably accessed.

## Validation before timing

- The candidate proves LUSK6 and exits zero in a direct run.
- All four focused allocator tests pass with default and all features,
  including parallel reuse.
- Strict all-feature library pedantic Clippy and formatting pass.
- The candidate executable is 9,078,784 bytes, 150,016 bytes larger than the
  8,928,768-byte parent.

The WSL distribution used for Callgrind became unavailable before this
experiment: `wsl` reported that no distributions were installed, and neither
Docker nor Podman was available. Native production timing was therefore used
as the first falsification gate. A rejected native result does not require
reinstalling the profiling environment or running compatibility matrices.

## Native result

All 128 measured processes prove and exit zero. Across all 64 pairs, the
candidate is worse:

- wall mean regresses 0.620105%, from 1.513436 to 1.522821 seconds;
- process-CPU mean regresses 0.297030%, from 1.479492 to 1.483887 seconds;
- wall and CPU medians regress 1.215321% and 1.063830%;
- mean paired wall and CPU changes regress 0.779752% and 0.449661%;
- median paired wall and CPU changes regress 0.893496% and 1.042119%;
- the candidate wins only 26 wall pairs and 29 CPU pairs, with two CPU ties.

The last 32 pairs do not recover wall performance:

- wall mean regresses 0.381950%, while CPU mean improves 0.392799%;
- wall median regresses 1.542975%, while CPU median is equal;
- mean paired wall regresses 0.550588%, while paired CPU improves 0.200473%;
- the candidate wins 15 wall pairs and 16 CPU pairs, with two CPU ties.

The measured rows are retained in `native-lusk.csv`. The saved warmup rows are
under the ignored artifact directory.

## Decision

Reject. Thread-local access and teardown support cost more in the production
Windows binary than the accepted uncontended spin lock, enlarge the executable,
and make the primary native metric worse. Restore Experiment 261 source
byte-for-byte. The accepted baseline remains 9,106,424,013 Callgrind
instructions, or 1.733117 times C.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-261-global-size-freelist\release\eprover.exe `
  -CandidateExe .\target\native-262-thread-local-size-freelist\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-024-thread-local-size-freelist\native-lusk.csv
```
