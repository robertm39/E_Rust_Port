# Windows generated-clause polling cadence

## Question

Does polling the Windows process CPU clock for every generated clause consume
material HEN011 proof-search margin, and can the query cadence be reduced
without reopening the BOO020/SWV851 allocation boundary?

## Setup

- Parent source: commit `1c5344f3` (`Reduce transient proof-search ownership`).
- Candidates: poll generated-clause admission at batch entry and then every 64,
  eight, or two clauses. The parent polls every clause.
- Native benchmark: three direct `HEN011-2.p` runs per build with
  `--auto --silent --cpu-limit=60 --memory-limit=2048 --detsort-rw
  --detsort-new --proof-object=1`.
- Compatibility corpora: the retained four-case GEO/HEN/LUSK proof corpus and
  two-case BOO/SWV resource corpus against archived C commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`.

Direct HEN samples are retained in `baseline-hen.csv`, `candidate-hen.csv`
(cadence 64), and `cadence8-hen.csv`. Direct BOO boundary samples are retained
in `cadence8-boo.csv` and `cadence2-boo.csv`.

## Results

All direct HEN runs prove `Unsatisfiable`, exit zero, and emit the same
24,943-byte proof output. Cadence 64 lowers median process CPU from 57.6875 to
56.703125 seconds, a 1.71% reduction, while median wall time falls 0.50%.
Cadence eight records a lower 53.15625-second CPU median, but the large change
relative to cadence 64 was measured in a later sequential block and is too
load/frequency-sensitive to attribute entirely to the branch.

The cadence-64 four-case proof report at
`.artifacts/e-compare/20260720-123234-981745/` has zero mismatches, including
the exact HEN normalized refutation. Its one-shot BOO/SWV report at
`.artifacts/e-compare/20260720-123440-472543/` also has zero mismatches: both
Rust cases return `ResourceOut`. That evidence is insufficient: the loaded
50-case report at `.artifacts/e-compare/20260720-124432-541033/` makes HEN
exact but aborts BOO on a 139,264-byte allocation at 59.35 seconds. The report
therefore has two unexpected rows, BOO and the known one-second LUSK cutoff,
plus the declared sledgehammer difference.

Tighter cadences do not restore the boundary reliably. Cadence eight returns
normal `ResourceOut` twice, then aborts on the same 139,264-byte allocation at
58.27 process-CPU seconds. Cadence two is worse in its measured block: one
normal `ResourceOut` followed by two 139,264-byte allocation aborts at 57.78
and 57.77 process-CPU seconds. A one-clause delay is enough to lose the race;
the parent per-clause check is required. A direct non-proof-object one-second
LUSK check remains `ResourceOut` for cadence 64, so none of the candidates
closes the short-budget performance gap.

Linux receives asynchronous `SIGXCPU`, so all candidates were non-Linux-only
and would leave deterministic Linux Callgrind execution unchanged.

## Falsification checks

- Every candidate checks at batch entry, so an already latched deadline is
  observed before admitting another clause.
- The focused resource corpus deliberately precedes repeated direct BOO runs
  and the full matrix; the latter two expose failures hidden by one-shot proof.
- Cadences 64, eight, and two bound the maximum observation delay to 63, seven,
  and one clause respectively. All three are rejected by allocator evidence.
- The proof corpus checks completion, exit behavior, proof shape, and normalized
  output rather than relying only on direct timing rows.

## Reproduction

```powershell
cargo build --locked --release --bin eprover `
  --target-dir target\native-146-cadence2
& .\experiments\2026-07-19-134-compact-clause-owners\measure_windows.ps1 `
  -Binary .\target\native-146-cadence2\release\eprover.exe `
  -Problem .\eprover\EXAMPLE_PROBLEMS\SMOKETEST\BOO020-1.p `
  -OutputCsv .\experiments\2026-07-20-146-windows-time-polling\cadence2-boo.csv `
  -Label cadence2 -Runs 3 -CpuLimit 60
```

## Decision

Reject every reduced admission cadence and retain per-clause Windows polling.
The process-clock query has measurable HEN cost, but even one unpolled admitted
clause can reopen the allocator abort that the parent source closes. Continue
performance work in allocation-free, cross-platform proof-search hot paths;
HEN remains close to the 60-second boundary and the one-second LUSK case still
does not prove.
