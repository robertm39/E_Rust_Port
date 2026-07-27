# Experiment 324: Pairwise small-arity term-top comparison

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the borrowed term-tree comparator dispatch on both `TermArgs`
representations together, unroll the inline zero/one/two-argument cases, and
retain a length-checked heap fallback, reducing the measured term-insertion
differential without changing storage layout or key order?

## Baseline

- Accepted production: Experiment 320 commit `2e2c5270`.
- Diagnostic checkpoint: Experiment 323 commit `cf4a6b02`.
- Matched accepted LUSK6 work: `7,605,982,425` instructions.
- `BorrowedTermCell::compare_top_order`: `189,198,884` self and `349,235,156`
  inclusive instructions over `7,113,427` calls in the line-table profile.
- Latest comprehensive aggregate: `1.114890x` C.

## Candidate

`TermArgs::compare_initialized_identity_order` dispatches on both argument
representations together. It returns arity order directly for unlike variants,
compares the single initialized slot for `One`, explicitly compares the two
slots for `Two`, and retains length-first iteration for two heap-backed
representations. This preserves the existing `TermArgs` storage, the
`TermCell` layout, uninitialized-slot panic contract for equal arities,
higher-order type ordering, and raw allocation-identity order.

The first encoding left the tiny
`initialized_term_identity_order` slot helper to ordinary compiler inlining.
Rust 1.97.1 kept it out of line. The final candidate uses one measured
`#[inline(always)]` directive on that new helper; it does not force-inline the
public argument API or repeat Experiment 300.

The owned comparator oracle now includes zero-, one-, two-, and three-argument
terms in both directions, differing heap arguments, and identity.

## Setup and exact commands

Retained focused evidence comes from dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-125122-5850` with Rust 1.97.1 and Callgrind 3.22.0.
The uploaded snapshot SHA-256 was
`f63c0ea31b7bc99df5dace5d1c60d2f095f4a6b6bed93cc343e28bd0ea0a38a7`.
The accepted parent archive SHA-256 was
`25F092A7238FFAD4C33BBAD6EFCC095F049B8A64316EFECA71DFC650AC8407C3`.
Measured source hashes were:

```text
7bada3f0a13872be03ed6ac18b8d2c7bd2b8ade3e836b67f858e294bcf052375  src/terms/termtrees.rs
b07d099cf276ce3e46c1a59b359fc1e267cb96b519f3584192c0d75e19beea92  src/terms/termtypes.rs
```

The focused lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar cf4a6b02 `
  src/terms/termtrees.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-023-pairwise-term-args-compare/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-324
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-023-pairwise-term-args-compare/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-324
    .\.venv\Scripts\python.exe -c `
      "import sys; sys.path.insert(0, r'tools/linode-runner'); import linode_runner as lr; state=lr.load_current(); state['remote_artifact_path']='/opt/e-rust-port/artifacts/experiment-324'; lr.save_current(state); print(lr.collect_artifacts(state))"
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
`e-rust-codex-260726-130726-4077`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- The pairwise comparator must match the retained owned oracle across zero,
  one, two, and heap argument shapes, both directions, and identity.
- Different arities must retain C's arity-first order without inspecting
  uninitialized argument slots.
- Equal-arity initialized terms must retain raw allocation-identity order.
- Exact LUSK6 proof output, status, stderr, and instructions must match or
  improve before native alternating measurement.
- Native timing is authoritative for acceptance.

## Results

Rustfmt, 25 focused tests, and strict all-target/all-feature pedantic Clippy
pass. Parent and final candidate exit zero with empty program stderr and
byte-identical LUSK6 proof output, SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
The candidate executable is 2,496 bytes smaller:

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Parent | `5a066925ef03` | 8,270,336 | 7,605,982,425 |
| Candidate | `4ef9d27bb6ba` | 8,267,840 | 7,508,052,140 |

Exact work falls by 97,930,285 instructions (`1.287543%`). Against Experiment
310's matched C total of `5,254,418,333`, the Rust/C instruction ratio moves
from `1.447540x` to `1.428903x`.

The intended comparator boundary falls from 349,237,436 to 251,210,480
instructions, down 98,026,956 (`28.0689%`), and accounts for essentially the
complete whole-program change.

The initial un-inlined encoding demonstrates why the directive is necessary:
the pairwise comparator body fell to 308,350,397 self instructions, but
8,370,069 calls to the new slot helper added 92,070,759 instructions. Total
work rose to 7,657,260,551, a 51,278,126-instruction (`0.674181%`)
regression. Its 64-pair native block also regressed paired mean wall/CPU by
`0.3881%`/`0.3875%`. That encoding is rejected.

Two independent retained blocks provide 128 alternating native pairs for the
final encoding. Every run has the exact proof hash. The candidate wins 93
wall and 92 CPU pairs and improves:

| Native metric | All 128 pairs | Combined stable 64 |
| --- | ---: | ---: |
| Paired mean wall | `0.621413%` | `0.579653%` |
| Paired mean CPU | `0.621662%` | `0.579329%` |
| Median wall | `0.621789%` | `0.709537%` |
| Median CPU | `0.610442%` | `0.706053%` |
| Candidate wall wins | 93 | 43 |
| Candidate CPU wins | 92 | 42 |

Fresh comprehensive run `.artifacts/linode/260726-130726-4077/` validates the
exact candidate binary SHA-256
`4ef9d27bb6bae9c46fd17de751cab1ca8b483bd85b8fe74f2473f7e489b4608c`:

- 4,416 Rust tests across 33 result groups, Rustfmt, strict pedantic Clippy,
  and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean pinned first-order and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior;
- smoke Callgrind records 10,002,383 Rust versus 7,590,630 C instructions;
  and
- the ten-case aggregate is `1.1138526684x` C, versus the accepted
  Experiment 320 fresh aggregate of `1.114890x`.

`VALIDATION_COMPLETE` and `SUCCESS` both contain `ok`. The worker and firewall
were deleted after artifact collection.

## Decision

Accept. Pairwise small-arity dispatch preserves the established layout,
ordering, proof, compatibility, resource, and portability contracts. The
measured inline refinement reduces the intended comparator 28.07%, exact
whole-program work 1.29%, and native time in both independent blocks and their
stable halves. The maintained aggregate remains above `1.10x`, so main-prover
performance parity remains open.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-023-pairwise-term-args-compare/experiment-324/
.artifacts/experiments/2026-07-25-023-pairwise-term-args-compare/remote.tar.gz
.artifacts/linode/260726-130726-4077/
```

The focused archive SHA-256 is
`B797C2BDC92F6977F70C1E1D729B88A8F28021625F03F6FB7E987AF198D650A0`.

## Limits and setup failures

- Explicit arity arms rely only on the private constructor invariant that
  `Empty`, `One`, `Two`, and `Heap` represent arities zero, one, two, and at
  least three. The retained heap path still compares actual lengths first.
- Different arities return before inspecting argument slots, preserving the
  C-compatible ability to compare an uninitialized top shell by arity.
  Equal-arity slots retain the existing initialization panic.
- The initial worker `e-rust-codex-260726-121750-8b66` reached
  `provision-failed` before Cargo was installed and contributed no evidence.
- Successful worker `e-rust-codex-260726-121946-e16d` first exposed the
  un-inlined regression and then measured the accepted refinement. Its console
  totals were valid and the refined exact totals match the retained
  reproduction byte-for-byte, but its remote artifacts were mistakenly not
  collected before deletion. The acceptance decision and all reported final
  statistics use the fresh retained reproduction
  `e-rust-codex-260726-125122-5850`; the loss is disclosed rather than treated
  as retained evidence.
- The comprehensive aggregate improved only `0.093%` relative across fresh
  workers, much less than the focused same-worker timing. Cross-worker
  aggregate movement is load-sensitive and is not the causal acceptance
  evidence.
