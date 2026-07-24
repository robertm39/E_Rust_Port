# Experiment 280: Borrowed rewrite-sequence argument pairs

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can structural rewrite-sequence construction recurse over paired borrowed
argument slices, matching C's direct `from->args[i]`/`tmp->args[i]` walk
without cloning two `Rc<TermCell>` handles per child edge?

## Setup

- Parent source: commit `25ba7155` (`perf: reject borrowed occurrence
  traversal`); executable source remains accepted Experiment 270.
- Fresh unchanged-source default-feature control: 8,991,960,325
  instructions.
- Archived accepted default-feature profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: borrow the source and replacement argument arrays for the
  duration of each structural rewrite link and pass child handles by
  reference. Owned top-link traversal and replacement acquisition remain
  unchanged.
- Variant B borrows only the source argument array and retains the accepted
  owned replacement child, testing whether one guard plus one clone transfers
  better than two guards across recursion.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted line profile attributes 163,712,831 instructions to 153,429
top-level `term_compute_rw_sequence` calls and 126,109,659 instructions to
284,810 recursive calls. Original C recurses directly through its two raw
argument arrays.

## Results

### Paired borrowed slices

The original compact and line-table candidate profiles were accidentally
built with `--all-features` and are not comparable with the default-feature
accepted baseline. A configuration audit measured the unchanged source at
9,078,864,096 instructions with all features versus 8,991,960,325 with
default features, a build-configuration penalty of 86,903,771 instructions
or 0.966461%. Those original whole-program and line-owner comparisons are
therefore superseded.

The corrected default-feature candidate executes 8,978,445,581 instructions.
That is 13,514,744 or 0.150298% below the fresh unchanged-source control and
14,367,344 or 0.159765% below the archived accepted profile. Its hypothetical
ratio to the 5,254,361,329-instruction C reference is 1.708761.

The all-feature native executable grows from 8,952,320 to 9,012,224 bytes.
More importantly, repeated direct native runs expose nondeterministic proof
output: two of three candidate runs match the parent's stable 10,350-byte
proof object, while one produces a different 10,208-byte proof object. All
runs exit zero and prove the expected result. This violates the maintained
exact proof-object compatibility contract despite the corrected instruction
win.

### Source-only borrowed slice

Variant B borrows only the source argument array while retaining the accepted
owned replacement child. Its original 9,073,151,795-instruction result was
also built with all features and is invalid as a comparison to the accepted
default-feature baseline. It was not rerun after the paired-slice candidate
failed deterministic proof compatibility.

## Validation

- All four candidate clause-position tests pass in default and all-feature
  modes.
- The existing recursive rewrite-sequence regression covers a structural root
  link and records the exact injected operation/demodulator stack order.
- Strict all-feature library pedantic Clippy and formatting pass for the
  paired-slice candidate.
- Corrected default-feature WSL Callgrind for the paired candidate proves
  LUSK6 and exits zero.
- Three direct native proof comparisons all exit zero; two are byte-identical
  to the parent and one differs.
- Native timing and the full maintained compatibility matrix are skipped
  after the repeated direct check rejects deterministic proof compatibility.
- After rejection, accepted `clausepos.rs` is restored byte-for-byte.

## Decision

Reject both variants. The paired-array candidate improves corrected
default-feature whole-program instructions by 0.150298%, but intermittently
changes the exact proof object on the maintained LUSK6 gate. Keep Experiment
270 as the accepted baseline at 8,992,812,925 instructions, or 1.711495 times
C.

Rewrite-sequence child ownership remains exhausted at this representation
boundary because the borrowed traversal is not proof-reproducible.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-007-borrowed-rw-sequence-args/rust-callgrind-borrowed-rw-sequence-args-default-corrected.out \
  target-wsl-280-corrected-borrowed-rw-sequence-args/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
