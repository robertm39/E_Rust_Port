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

### Native timing

The corrected native candidate preserves byte-identical proof-object output,
but decisively reverses the instruction win. After four alternating warmup
pairs, 64 alternating measured pairs show:

- wall and CPU means regress 7.831316% and 7.383242%;
- wall and CPU medians regress 7.853853% and 7.865169%;
- mean paired wall and CPU changes regress 7.944031% and 7.487450%;
- the candidate wins only one wall pair and one CPU pair, with no CPU ties.

The final 32 pairs remain negative at 7.098325% wall and 7.105171% CPU by
aggregate means. The final 16 regress 8.258772% wall and 7.610994% CPU, with
zero candidate wins. All 128 measured and eight warmup processes exit zero.
The all-feature native executable grows from 8,952,320 to 9,012,224 bytes.

The previously attempted no-inline variant used the invalid all-feature
Callgrind configuration and is not used in the decision. It is not rerun
because the original corrected candidate already fails the native gate by
more than seven percent.

## Validation

- All 21 default and 22 all-feature MGU tests pass for the borrowed candidate.
- A focused regression covers a multi-link borrowed binding chain leading
  through an ordinary term argument.
- Strict all-feature library pedantic Clippy and formatting pass for Variant
  A.
- The corrected default-feature WSL Callgrind run proves LUSK6 and exits zero.
- Direct native parent/candidate proof-object output is byte-identical.
- All corrected native timing processes prove and exit zero.
- Compatibility matrices are skipped after the decisive native rejection.
- After rejection, the binding borrow visibility, occurrence-check
  implementation, and focused regression are removed and accepted sources
  are restored byte-for-byte.

## Decision

Reject. Borrow-scoped occurrence checking produces a genuine 0.437764%
default-feature instruction reduction, but native production slows by more
than seven percent across the full sample and stable tails. Keep Experiment
270 as the accepted baseline at 8,992,812,925 instructions, or 1.711495 times
C.

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
  -CandidateExe .\target\native-279-corrected-borrowed-occur-check\release\eprover.exe `
  -Pairs 64 `
  -OutputCsv .\experiments\2026-07-24-006-borrowed-occur-check\native-lusk-corrected.csv
```
