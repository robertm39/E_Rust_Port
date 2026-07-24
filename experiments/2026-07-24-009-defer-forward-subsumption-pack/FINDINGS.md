# Experiment 282: Defer forward-subsumption packing

## Status

Rejected for Bead `E_Rust_Port-j76.5.3`.

## Question

Can production forward contraction consume only the subsumed/not-subsumed
decision and defer Rust's owned `FvPackedClause` construction until the final
clause state, avoiding a clause/frequency-vector clone that both production
callers immediately discard?

## Setup

- Parent source: commit `21f263e0` (`perf: reject no-doc forward
  specialization`); executable source remains accepted Experiment 270.
- Accepted compact profile: 8,992,812,925 instructions.
- Representative optimized line profile:
  `.artifacts/experiments/2026-07-23-033-pdt-cursor-after-active-frame/rust-callgrind-pdt-cursor-after-active-frame.out`.
- Parent native executable:
  `target/native-270-borrow-active-pdt-frame/release/eprover.exe`.
- Candidate: factor the bank-aware subsumption decision into a private Boolean
  helper. The public packed-return API retains its exact contract, while
  ordinary and aggressive production contraction use the decision directly.
- Variant B prevents the factored decision helper from being duplicated into
  its packed and decision-only callers, testing whether code expansion causes
  any whole-program reversal.
- Workload: upstream `LUSK6.lop` with `--auto --silent --cpu-limit=600
  --memory-limit=2048 --detsort-rw --detsort-new`.

Original C's `FVPackedClause` aliases the live clause and is retained through
later selection/maximality mutations. Rust's safe packed value owns a clause
clone, so the existing production caller discards the early clone and packs
again after mutation. The accepted profile records 1,700 bank-aware
forward-subsumption calls and about 14,030,040 inclusive instructions at this
boundary.

## Results

### Factored decision

The candidate preserves the expected unsatisfiable result and produces a
byte-identical native proof object. It nevertheless rises from 8,992,812,925
to 9,074,044,057 exact instructions, a regression of 81,231,132 or
0.903289%. The hypothetical Rust/C ratio worsens from 1.711495 to 1.726955.

The all-feature native executable grows from 8,952,320 to 9,010,176 bytes, an
increase of 57,856 bytes. Although production avoids the early owned packed
clause, factoring the decision changes optimized code layout far beyond the
roughly 14-million-instruction source owner.

### Out-of-line decision

Variant B applies `#[inline(never)]` to the factored decision helper. It
improves Variant A by only 324,936 instructions and still retires
9,073,719,121 instructions: 80,906,196 or 0.899676% above the parent.
Containing helper duplication therefore does not recover the ownership
saving.

## Validation

- All 219 proof-control tests pass in default and all-feature configurations.
- The public packed-return regression retains its exact contract.
- Strict all-feature library pedantic Clippy and formatting pass for Variant
  A.
- Exact WSL Callgrind for both variants proves LUSK6 and exits zero.
- Direct native parent/candidate proof-object output is byte-identical.
- Native timing and compatibility matrices are skipped after both
  exact-instruction profiles reject the performance-only change.
- After rejection, the factored helper and decision-only private return are
  removed and accepted `proofcontrol.rs` is restored byte-for-byte.

## Decision

Reject both variants. Removing the immediately discarded Rust-owned pack is
locally sound and proof-exact, but the required control-flow refactor produces
a much larger optimized-layout regression. Keep Experiment 270 as the
accepted baseline at 8,992,812,925 instructions, or 1.711495 times C.

Forward-subsumption packing should retain the accepted code shape until clause
ownership can represent C's stable alias directly; helper factoring and forced
out-of-lining both lose at whole-program scope.

## Reproduction

```bash
valgrind --tool=callgrind \
  --callgrind-out-file=.artifacts/experiments/2026-07-24-009-defer-forward-subsumption-pack/rust-callgrind-defer-forward-subsumption-pack.out \
  target-wsl-282-defer-forward-subsumption-pack/release/eprover \
  eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```
