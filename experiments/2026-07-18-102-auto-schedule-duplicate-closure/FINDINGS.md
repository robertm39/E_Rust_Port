# Auto-schedule State-transfer and Resource Closure

## Status

Completed for Bead `E_Rust_Port-j76.2.35`. A previous P2 scheduler bead had
already implemented parent-request handling, stdin snapshot replay, the
two-clock retry calculation, and the safe exec-worker state-transfer decision.
A fresh unchanged-C comparison found and fixed three remaining observable
handoff/accounting defects. The vendored C checkout remained unchanged.

## Fresh defects and fixes

### Parent-assigned preprocessing cell state

C mutates the selected preprocessing `ScheduleCell` before `fork()`. The child
therefore inherits its absolute CPU budget and assigned core count, and uses
those values to initialize the nested search schedule.

Rust already transferred the selected name, ordering, and absolute budget to
the exec worker, but rebuilt the generated preprocessing schedule without
rehydrating the last two mutable fields. The nested search consequently used
the generated one-second placeholder and scheduled only one search strategy.
The private worker protocol now carries the assigned core count too, and the
worker restores both `time_absolute` and `cores` on the selected cell. The
reference fixture now schedules one preprocessing strategy for 300 seconds and
six search strategies for the same 300-second total, like C.

### Resource-footer ownership

C sets `SilentTimeOut` in a schedule leaf. The proof-search leaf suppresses its
ordinary final resource footer; the nested search coordinator and outer
preprocessing coordinator each add one footer after replaying their winner.
The reference therefore emits two post-proof resource summaries.

The Rust search leaf previously emitted its ordinary footer before the two
coordinator layers, producing three. Hidden search workers now suppress only
that leaf footer. The remaining two totals are nondecreasing because waited
child usage is propagated through each exec boundary.

### Scheduled CNF interaction

C enters the nested schedule when `strategy_scheduling` selected a
preprocessing child even if `--cnf` is also set. Its zero processed-clause limit
still permits preprocessing/presaturation to discover an existing
contradiction. Rust used its ordinary CNF-only early return in hidden workers,
so the coordinator saw no recognized proof result and incorrectly exhausted
the schedule. Hidden preprocessing/search workers now follow the scheduled C
path, while ordinary unscheduled CNF-only runs keep their existing boundary.

## Portable state-transfer decision

The retained decision from
[`experiments/2026-07-16-034-multicore-fork-compatibility/FINDINGS.md`](../2026-07-16-034-multicore-fork-compatibility/FINDINGS.md)
still applies. C schedule children inherit the complete pointer-rich parser,
term-bank, proof-state, and preprocessing heap through copy-on-write `fork()`.
Continuing allocation-heavy Rust after `fork()` would violate the project safe
Rust rule and is unavailable on Windows. A complete versioned heap-graph
serialization format would add a larger compatibility and maintenance surface
than the measured startup cost justifies.

Rust therefore deliberately uses portable exec workers, snapshots ephemeral
standard input once, and reparses stable named files. The private protocol now
transfers every schedule-cell field that affects nested strategy selection,
budgeting, and core allocation. PID/timing text and C stdio-buffer duplication
across `fork()` remain intentionally outside exact text comparison; selected
classes, schedules, strategies, results, and resource-layer semantics are
exact.

## Exact C/Rust projection

[`compare_schedule.py`](compare_schedule.py) runs isolated unchanged C at
commit `17026b1bfe61aaf223cfaae54947c8d2679c31a0` and the Rust release on a
false-clause fixture, both with and without `--cnf`. Both cases are exact for:

- preprocessing class `FSSSSMSSSSSNFFN` and search class
  `FUHPF-FFSF00-SFFFFFNN`;
- schedule summaries `1 strategy / 1 core / 300 seconds` followed by
  `6 strategies / 1 core / 300 seconds`;
- preprocessing winner `G-E--_302_C18_F1_URBAN_RG_S04BN` and search winner
  `SAT001_MinMin_p005000_rr_RG`;
- preprocessing-time presence, proof completion, `SZS status Unsatisfiable`,
  empty stderr, and exit `0`; and
- exactly two resource footers after the proof with nondecreasing aggregate
  totals.

[`audit_scheduler.py`](audit_scheduler.py) passes 19/19 source, protocol,
owner, accounting, permanent-regression, prior-decision, and fresh-reference
checks.

The retained [`reference.json`](reference.json) has SHA-256
`6F16610EA581A6DB2657BEE2A5E05FCFA96F1285E27DDDB758EDABA7323DAB7A`.
The retained [`owner-audit.json`](owner-audit.json) has SHA-256
`EA8A5D6B54FC55A10262A69C1AC3486972D8055A15DFF385CFAF0D23C6CABCCF`.
The compared Rust `eprover.exe` has SHA-256
`E002036361411C0846863E0F460C0A4BFCB4BB534F90B673E5EFD55D42716821`.

## Reproduction

```powershell
cargo build --locked --release --bin eprover --all-features
cargo test --locked --all-features --test eprover_schedule --quiet

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-102-auto-schedule-duplicate-closure\audit_scheduler.py `
  --reference experiments\2026-07-18-102-auto-schedule-duplicate-closure\reference.json `
  --output target\auto-schedule-owner-audit.json

C:\Users\rober\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe `
  experiments\2026-07-18-102-auto-schedule-duplicate-closure\compare_schedule.py `
  --c-exe /home/rober/.cache/e-rust-port/bin/17026b1bfe61aaf223cfaae54947c8d2679c31a0/fol/eprover `
  --rust-exe target\release\eprover.exe `
  --output target\auto-schedule-reference.json `
  --expected experiments\2026-07-18-102-auto-schedule-duplicate-closure\reference.json
```

## Compatibility decision

The observable scheduler state-transfer and resource-accounting gap is closed.
Portable exec workers are the compelling safe/cross-platform replacement for
C copy-on-write heap inheritance; all schedule-cell state needed downstream is
now explicit, and the retained coordinator projections match C in both normal
and scheduled-CNF proof cases.
