# Experiment 281: Specialize no-documentation forward modification

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can the production saturation path statically select the ordinary
`ForwardModifyClause` implementation instead of routing every generated clause
through the runtime proof-documentation `Option` dispatcher?

## Setup

- Parent source: commit `12d78512` (`perf: reject borrowed rewrite sequence
  arguments`); executable source remains accepted Experiment 270.
- Fresh unchanged-source default-feature control: 8,991,960,325
  instructions.
- Archived accepted default-feature profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: const-specialize the generated-clause admission and
  forward-contraction implementations on whether a proof-documentation session
  exists. Their shared forward-modification dispatcher calls the ordinary
  implementation directly in the production specialization and preserves the
  documented implementation in the opt-in specialization.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

The accepted profile sends 121,036 ordinary forward modifications through the
generated-clause loop plus 5,130 through forward contraction. The complete
generated-clause admission owner accounts for 4,705,517,773 inclusive
instructions, so this experiment changes only its repeated documentation
dispatch boundary, not contraction semantics.

## Results

The original 9,069,312,582-instruction candidate profile was accidentally
built with `--all-features` and is not comparable with the default-feature
accepted baseline. A configuration audit measured the unchanged source at
9,078,864,096 instructions with all features versus 8,991,960,325 with
default features, a build-configuration penalty of 86,903,771 instructions
or 0.966461%. The original rejection metric is therefore superseded.

The corrected default-feature candidate executes 8,957,241,257 instructions.
That is 34,719,068 or 0.386112% below the fresh unchanged-source control and
35,571,668 or 0.395557% below the archived accepted profile. Its hypothetical
ratio to the 5,254,361,329-instruction C reference improves to 1.704725.

The all-feature native executable grows from 8,952,320 to 8,979,968 bytes, an
increase of 27,648 bytes.

Repeated native proof runs reject the candidate on deterministic
compatibility. The parent produces one stable 33,636-character proof object.
Five candidate runs produce multiple proof objects of 33,636, 34,241, and
34,417 characters; only one of the five is byte-identical to the parent.
All runs exit zero and prove the expected result, but the differing clauses
are substantive rather than timing or summary text.

## Validation

- All 219 proof-control tests pass in default and all-feature configurations.
- Strict all-feature library pedantic Clippy and formatting pass.
- Corrected default-feature WSL Callgrind proves LUSK6 and exits zero.
- Three parent and five candidate direct native proof runs all exit zero; the
  parent is stable while the candidate emits multiple proof objects.
- Native timing and the full maintained compatibility matrix are skipped
  after the repeated direct check rejects deterministic proof compatibility.
- After rejection, the const parameters and specialized dispatch are removed
  and accepted `proofcontrol.rs` is restored byte-for-byte.

## Decision

Reject. Const-specializing the ordinary documentation path improves corrected
default-feature instructions by 0.386112%, but changes proof search and emits
nondeterministic proof objects on the maintained LUSK6 gate. Keep Experiment
270 as the accepted baseline at 8,992,812,925 instructions, or 1.711495 times
C.

The no-documentation specialization boundary is exhausted because its code
layout changes are not proof-reproducible.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-008-specialize-no-doc-forward-modify/rust-callgrind-specialize-no-doc-forward-modify-default-corrected.out \
  target-wsl-281-corrected-no-doc-specialization/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
