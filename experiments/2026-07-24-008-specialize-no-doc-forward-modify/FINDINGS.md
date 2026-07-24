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
- Accepted compact profile: 8,992,812,925 instructions.
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

The candidate preserves the expected unsatisfiable result and produces a
byte-identical native proof object. It nevertheless rises from 8,992,812,925
to 9,069,312,582 exact instructions, a regression of 76,499,657 or
0.850676%. The hypothetical Rust/C ratio worsens from 1.711495 to 1.726054.

The all-feature native executable grows from 8,952,320 to 8,979,968 bytes, an
increase of 27,648 bytes. Const-specializing the documentation mode duplicates
enough of the large generated-clause admission implementation to harm code
layout and instruction-cache locality far more than the removed `Option`
dispatch can save.

## Validation

- All 219 proof-control tests pass in default and all-feature configurations.
- Strict all-feature library pedantic Clippy and formatting pass.
- Exact WSL Callgrind proves LUSK6 and exits zero.
- Direct native parent/candidate proof-object output is byte-identical.
- Native timing and compatibility matrices are skipped after the decisive
  exact-instruction rejection.
- After rejection, the const parameters and specialized dispatch are removed
  and accepted `proofcontrol.rs` is restored byte-for-byte.

## Decision

Reject. The ordinary documentation `Option` branch is cheaper than duplicating
the large admission and contraction bodies to eliminate it. Keep Experiment
270 as the accepted baseline at 8,992,812,925 instructions, or 1.711495 times
C.

The no-documentation specialization boundary is exhausted. Future
proof-control work should target a concrete owned operation inside the shared
body rather than cloning the whole orchestration path for branch elimination.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-008-specialize-no-doc-forward-modify/rust-callgrind-specialize-no-doc-forward-modify.out \
  target-wsl-281-specialize-no-doc-forward-modify/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
