# Experiment 279: Borrowed occurrence-check traversal

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can first-order occurrence checking follow variable bindings and term arguments
under scoped borrows, matching C's raw-pointer recursion without acquiring an
owned `Rc<TermCell>` handle at every visited node?

## Setup

- Parent source: commit `9fb0c740` (`perf: reject changed-only recursive
  normalization`); executable source remains accepted Experiment 270.
- Accepted compact profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: expose the existing crate-private binding borrow, follow
  free-variable chains recursively under that borrow, and traverse ordinary
  argument slots by reference. Bound applied-variable heads retain the
  existing full-dereference expansion path.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted line profile attributes 138,342,949 instructions to 320,224
first-order `occur_check` entries and another 98,899,089 instructions to
430,524 recursive entries. The accepted implementation calls general
`term_deref` and clones each recursive argument; C `OccurCheck` follows raw
binding and argument pointers.

## Results

### Configuration correction

The initial compact and line-table candidates were accidentally built with
`--all-features` and compared to the default-feature accepted profile. The
unchanged-source audit in Experiment 283 shows that configuration adds
86,903,771 instructions, so the originally recorded whole-program regression
and local attribution are invalid and are superseded here.

The corrected default-feature candidate proves the expected unsatisfiable
result at 8,952,596,764 instructions. It improves the fresh unchanged-source
control of 8,991,960,325 by 39,363,561 or 0.437764%, and improves the archived
accepted profile by 40,216,161 or 0.447203%. The hypothetical Rust/C ratio is
1.703841.

### Native configuration correction and matched timing

The first native correction was also configuration-invalid: its candidate
fingerprint contained all features while the accepted parent fingerprint
contains only `features=["default"]`. Experiment 285 measures unchanged
all-feature source 8.832227% slower in wall time and 8.972648% slower in CPU,
so the previously recorded seven-percent candidate slowdown is superseded.

The matched default-feature candidate is 8,983,552 bytes versus 8,952,320 for
the parent. Three parent and five candidate direct proof runs are
byte-identical and exit zero.

Two independent native blocks each exclude four alternating warmup pairs and
retain 64 alternating measured pairs:

| Block | Wall mean | CPU mean | Wall wins | CPU wins | CPU ties |
| --- | ---: | ---: | ---: | ---: | ---: |
| First 64 pairs | +1.131783% | +0.569106% | 24 | 28 | 3 |
| Second 64 pairs | +0.498696% | +0.545815% | 33 | 26 | 7 |
| Combined 128 pairs | +0.817666% | +0.557560% | 57 | 54 | 10 |

Positive percentages are candidate regressions. Combined wall and CPU medians
regress 0.855621% and tie; mean paired wall and CPU changes regress 0.981794%
and 0.688592%.

The combined final halves remain negative at 0.807451% wall and 0.361011% CPU
by aggregate means. The second block's final 32 regress 0.344689% wall while
improving CPU 0.131883%; its final 16 tie wall within 0.006180% and improve CPU
0.727995%. Those late CPU gains do not overcome both full independent blocks
and the combined stable-half wall/CPU regressions.

The previously attempted no-inline variant used the invalid all-feature
Callgrind configuration and is not used in the decision.

## Validation

- All focused MGU tests pass for the borrowed candidate in default and
  all-feature modes.
- A focused regression covers a multi-link borrowed binding chain leading
  through an ordinary term argument.
- Strict all-feature library pedantic Clippy and formatting pass for Variant
  A.
- The corrected default-feature WSL Callgrind run proves LUSK6 and exits zero.
- Repeated matched default-feature native parent/candidate proof output is
  byte-identical.
- All 256 matched-feature measured timing processes and 16 warmup processes
  prove and exit zero.
- Compatibility matrices are skipped after the decisive native rejection.
- After rejection, the binding borrow visibility, occurrence-check
  implementation, and focused regression are removed and accepted sources
  are restored byte-for-byte.

## Decision

Reject. Borrow-scoped occurrence checking produces a genuine 0.437764%
default-feature instruction reduction, but matched native production regresses
wall and CPU means in two independent blocks and their combined stable halves.
Keep Experiment 270 as the accepted baseline at 8,992,812,925 instructions,
or 1.711495 times C.

Future first-order MGU work should preserve the accepted occurrence-check
code shape; the owned queue, specialized dereference, inline job deque, and
borrowed occurrence traversal have now all been isolated.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-006-borrowed-occur-check/rust-callgrind-borrowed-occur-check-default-corrected.out \
  target-wsl-279-corrected-borrowed-occur-check/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

```powershell
& .\experiments\2026-07-22-214-move-termtree-insert-links\run-native.ps1 `
  -ParentExe .\target\native-270-borrow-active-pdt-frame\release\eprover.exe `
  -CandidateExe .\target\native-279-default-borrowed-occur-check\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-006-borrowed-occur-check\native-lusk-default.csv
```

Run the native command twice independently; the second retained block is
`native-lusk-default-2.csv`. The older `native-lusk-corrected.csv` is
superseded by Experiment 285's native feature audit.
