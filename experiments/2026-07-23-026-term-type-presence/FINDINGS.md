# Experiment 264: Non-owning term-type presence check

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can `TermBank::term_top_insert` test whether a term has a type without cloning
and dropping the stored `Rc<Type>` on every insertion?

## Setup

- Parent source: commit `e53a9fb7` (`perf: cache exact-size allocations`),
  accepted Experiment 261.
- Parent native executable:
  `target/native-261-global-size-freelist/release/eprover.exe`.
- Candidate native executable:
  `target/native-264-term-type-presence/release/eprover.exe`.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.
- Timing protocol: two independent blocks, each with four alternating warmup
  pairs and 64 alternating measured pairs.

The accepted profile records 2,479,632 `term_top_insert` calls. The current
`term.type_().is_none()` check borrows the links, clones the optional
reference-counted type handle, tests it, and drops the clone. C tests the
nullable type pointer directly.

## Candidate

Add a crate-private `Term::has_type()` that borrows the links and returns
`type_.is_some()` without cloning the handle. Use it only at the type-inference
guard in `term_top_insert`; every owning type accessor and all type-inference,
duplicate, metadata, and higher-order behavior remain unchanged.

A focused regression checks the false/true transition around `set_type`.

## Validation before timing

- The candidate proves LUSK6 and exits zero in a direct run.
- One focused term-link test and all 122 term-bank tests pass.
- Strict all-feature library pedantic Clippy and formatting pass.
- Parent and candidate executables are both 8,928,768 bytes.
- All 256 measured processes prove and exit zero.

The WSL profiling distribution remained unavailable, and neither Docker nor
Podman was installed. Native production timing was therefore the first
falsification gate.

## Native result

Block one is mixed:

- aggregate wall and CPU means improve 0.017320% and 0.311322%;
- mean paired wall regresses 0.132608%, while paired CPU improves 0.136503%;
- the stable last 32 improve 0.660129% wall and 1.014398% CPU by aggregate
  means;
- wins split exactly 32/64 wall, with 30 CPU wins and four ties.

The independent second block does not reproduce a mean improvement:

- aggregate wall and CPU means regress 0.064639% and 0.085616%;
- mean paired wall and CPU regress 0.121648% and 0.136963%;
- the stable last 32 improve aggregate means by 0.872685% wall and 0.393279%
  CPU, but median wall/CPU changes remain flat to slightly negative;
- the candidate wins 30 wall pairs and 24 CPU pairs, with 11 CPU ties.

Across all 128 pairs, the candidate is effectively neutral and slightly
wall-negative:

- wall mean regresses 0.022867%, from 1.491863 to 1.492204 seconds;
- CPU mean improves 0.117223%, from 1.457886 to 1.456177 seconds;
- wall median regresses 0.183516%, while CPU median is equal;
- mean paired wall regresses 0.127128%, while paired CPU changes by only
  -0.000230%;
- paired wall and CPU medians are effectively zero;
- the candidate wins 62 wall pairs and 54 CPU pairs, with 15 CPU ties.

The measured rows are retained in `native-lusk.csv` and `native-lusk-2.csv`;
warmup rows are under the ignored artifact directory.

## Decision

Reject. Avoiding the temporary type-handle clone is semantically sound and
does not change binary size, but two complete native blocks establish no
production throughput improvement. Restore Experiment 261 source
byte-for-byte. The accepted baseline remains 9,106,424,013 Callgrind
instructions, or 1.733117 times C.

## Reproduction

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-261-global-size-freelist\release\eprover.exe `
  -CandidateExe .\target\native-264-term-type-presence\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-23-026-term-type-presence\native-lusk.csv
```
