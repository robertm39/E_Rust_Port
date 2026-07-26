# Experiment 325: Small-arity substitution argument push

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can substitution normalization push zero-, one-, and two-argument
`TermArgs` shapes directly in reverse, retaining the heap iterator fallback
and exact binding order while removing generic slice/iterator work from the
remaining matched normalization differential?

## Baseline

- Accepted production: Experiment 324 commit `cc5d1126`.
- Matched accepted LUSK6 work: `7,508,052,140` instructions.
- Experiment 323 attributed 67,986,964 `termtypes.rs` and 25,588,114
  `subst.rs` self instructions to `Substitution::norm_term` before the
  Experiment 324 comparator-only change.
- Latest comprehensive aggregate: `1.1138526684x` C.

## Candidate

`BorrowedTermCell::push_arguments_reversed`, used only by substitution
normalization, now matches directly on the private `TermArgs` representation.
It performs no work for `Empty`, conditionally pushes the one inline slot,
explicitly pushes inline slot one then zero for `Two`, and retains the generic
reverse iterator for heap-backed arities. Optional empty slots retain their
existing skip behavior.

The focused normalization regression now nests a unary term inside a binary
term and requires the substitution binding order `[x, y]`. Existing focused
tests retain the three-argument heap order, existing binding chains,
applied-variable expansion ownership, caught-panic scratch cleanup, and
backtracking.

## Setup and exact commands

Focused measurement used dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-133754-c498` with Rust 1.97.1 and Callgrind 3.22.0. The
uploaded snapshot SHA-256 was
`dd28e39f435b0e84b5a2309eb9b2423dcfd400bd286e5ace09d3450c2a9b183b`;
the accepted parent archive SHA-256 was
`01EE29EC1A3BD0B68AF79E9A562A753D081B6DCD5D522DAF20F97B15775C3BCF`.
Candidate source hashes were:

```text
3581f83c097ddaf723024e0d34fd8eb5431b0750fa42b4c85f1a9f78cfab884e  src/terms/subst.rs
62b1a955abef51efeda275c412abba1576c060603dd56baf9c7167096a0a503a  src/terms/termtypes.rs
```

The focused lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar cc5d1126 `
  src/terms/subst.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-024-small-arity-subst-push/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-325
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-024-small-arity-subst-push/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-325
    .\.venv\Scripts\python.exe -c `
      "import sys; sys.path.insert(0, r'tools/linode-runner'); import linode_runner as lr; state=lr.load_current(); state['remote_artifact_path']='/opt/e-rust-port/artifacts/experiment-325'; lr.save_current(state); print(lr.collect_artifacts(state))"
}
finally {
    .\linode-runner.ps1 down
}
```

Both exact profiles used:

```bash
valgrind --tool=callgrind \
  BINARY eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

Fresh exact-source comprehensive validation used worker
`e-rust-codex-260726-135431-1854`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- Normalization must preserve left-to-right first-binding order through empty,
  unary, binary, and heap-backed argument representations.
- Missing optional slots must retain the existing skip behavior.
- Applied-variable expansion ownership, binding-chain traversal, caught-panic
  scratch cleanup, and backtracking must pass the focused suite.
- Exact proof work must improve and the proof/status/stderr must remain exact
  before native alternating measurement is accepted.
- Native timing is authoritative.

## Results

Rustfmt, all 13 focused substitution tests, and strict
all-target/all-feature pedantic Clippy pass. Parent and candidate exit zero
with empty program stderr and byte-identical LUSK6 proof output, SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Parent | `4ef9d27bb6ba` | 8,267,840 | 7,507,939,039 |
| Candidate | `203423eda2cf` | 8,267,936 | 7,474,534,715 |

Exact same-worker work falls by 33,404,324 instructions (`0.444920%`).
Against Experiment 310's matched C count of `5,254,418,333`, the instruction
ratio moves from `1.428881x` to `1.422524x`. The candidate executable grows
by 96 bytes.

The intended `Substitution::norm_term` owner falls from 343,678,069 to
310,162,887 self instructions, down 33,515,182 (`9.7519%`), and explains
essentially the complete whole-program change. Both binaries make the same
488,212 normalization calls.

Two independent 64-pair blocks provide 128 alternating native pairs. Every
run has the exact proof hash. The candidate wins 72 wall and 72 CPU pairs and
improves:

| Native metric | All 128 pairs | Combined stable 64 |
| --- | ---: | ---: |
| Paired mean wall | `0.210471%` | `0.175915%` |
| Paired mean CPU | `0.210606%` | `0.175611%` |
| Paired median wall | `0.242634%` | `0.203188%` |
| Paired median CPU | `0.240002%` | `0.209074%` |
| Candidate wins | 72 | 37 |

Fresh comprehensive run `.artifacts/linode/260726-135431-1854/` validates the
exact candidate binary SHA-256
`203423eda2cf63e06886bd0052a7d289efd8661da685461d64bb7498f5ed75a2`:

- 4,417 Rust tests across 33 result groups, Rustfmt, strict pedantic Clippy,
  and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean pinned first-order and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior;
- smoke Callgrind records 9,999,856 Rust versus 7,590,616 C instructions; and
- the ten-case aggregate improves from Experiment 324's `1.1138526684x` to
  `1.1119780745x` C.

`VALIDATION_COMPLETE` and `SUCCESS` both contain `ok`. Both workers and
firewalls were deleted after artifact collection.

## Decision

Accept. The candidate preserves every argument representation, optional-slot
behavior, traversal order, ownership boundary, proof, compatibility,
portability, and resource result. It reduces normalization self work 9.75%,
whole-program exact work 0.445%, and native time in both independent blocks
and their stable halves. The maintained aggregate remains above `1.10x`, so
main-prover performance parity remains open.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-024-small-arity-subst-push/experiment-325/
.artifacts/experiments/2026-07-25-024-small-arity-subst-push/remote.tar.gz
.artifacts/linode/260726-135431-1854/
```

The focused archive SHA-256 is
`D5D793205A7C1B4E1F1BC0AFFCA071987F1C6397AD6D694250B0A9489B73982C`.

## Limits

- The direct inline arms deliberately preserve `flatten`'s behavior by
  conditionally pushing each optional slot rather than assuming
  initialization.
- The private `TermArgs` constructor remains the sole owner of the
  zero/one/two/heap shape invariant; no layout or public API changes.
- The same accepted parent executable SHA-256 recorded 7,508,052,140
  instructions in Experiment 324 and 7,507,939,039 here, a
  113,101-instruction (`0.00151%`) cross-worker/runtime-startup variation.
  The candidate decision uses the same-worker parent/candidate delta.
- The comprehensive aggregate is load-sensitive across fresh workers. Its
  direction agrees with the retained exact and paired evidence, but it is
  context rather than the causal acceptance measurement.
