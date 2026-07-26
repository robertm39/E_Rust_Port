# Experiment 326: Small-arity KBO argument push

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the first-order KBO variable-balance traversal push zero-, one-, and
two-argument `TermArgs` shapes directly in reverse, retaining the heap
iterator fallback and exact dereference mode while removing generic
slice/iterator work from the remaining KBO differential?

## Baseline

- Accepted production: Experiment 325 commit `df7943ab`.
- Matched accepted LUSK6 work: `7,474,421,606` instructions on this worker.
- Experiment 316's borrowed KBO cursor left
  `BorrowedTermCell::push_first_order_arguments_reversed` mapping every
  `TermArgs` shape through `as_slice().iter().rev().flatten()`.
- Latest comprehensive aggregate: `1.1119780745x` C.

## Candidate

`BorrowedTermCell::push_first_order_arguments_reversed`, used only by
first-order KBO's `mfy_vwb`, now matches directly on the private `TermArgs`
representation. It performs no work for `Empty`, conditionally pushes the one
inline slot, explicitly pushes inline slot one then zero for `Two`, and
retains the generic reverse iterator for heap-backed arities. Every pushed
cursor receives the unchanged caller-supplied dereference mode.

A direct regression keeps owners live while it verifies reverse stack order
and dereference mode across arities zero through four. Existing KBO tests
retain first-order/classic result equivalence, inline binary traversal,
variable balance, scratch reuse, and caught-panic stale-cursor cleanup.

## Setup and exact commands

Focused measurement used dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-142839-42ea` with Rust 1.97.1 and Callgrind 3.22.0. The
successful uploaded snapshot SHA-256 was
`bbe6c5a0dc0da4fb6af8d9d0ac368131e08a934d3effdbfc96de136b9bc79dfb`;
the accepted parent archive SHA-256 was
`BFDF6B76ADBD6D2FE3894AAE9B0D51AD5BC35BF67C38E6E37E9C8E789057D3A2`.
The final candidate source hash was:

```text
951edb9cdf94208cdd33db6c94fa2d06352357d2cd0b4af740d99738361dda6d  src/terms/termtypes.rs
```

The focused lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar df7943ab `
  src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-025-small-arity-kbo-push/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-326
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-025-small-arity-kbo-push/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-326
    .\.venv\Scripts\python.exe -c `
      "import sys; sys.path.insert(0, r'tools/linode-runner'); import linode_runner as lr; state=lr.load_current(); state['remote_artifact_path']='/opt/e-rust-port/artifacts/experiment-326'; lr.save_current(state); print(lr.collect_artifacts(state))"
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
`e-rust-codex-260726-144607-93c2`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- The direct regression must preserve right-to-left push order through empty,
  unary, binary, and heap-backed argument representations, and every cursor
  must retain the shared dereference mode.
- KBO variable balance, first-order comparison results, scratch reuse, and
  caught-panic cleanup must remain exact.
- Exact proof work must improve and proof/status/stderr must remain exact
  before native alternating measurement is accepted.
- Both independent native timing blocks must have a positive paired-mean
  direction; native timing remains authoritative.

## Iteration history

An initial controller invocation was given an accidentally short local
timeout. It created `e-rust-codex-260726-142801-88a7` but was interrupted
before bootstrap or source upload; the exact Linode and firewall were deleted
before the successful worker was provisioned. It produced no experiment data.

The first source upload stopped at Rustfmt before compilation because the new
test call needed the formatter's continuation indentation. The second upload
stopped at the crate-wide `unsafe_code` lint because the direct regression
intentionally invokes the internal unsafe borrowed-cursor API. The final
source adds a reasoned function-local lint allowance and adjacent safety
comments while keeping all cursor owners alive. Neither pre-gate iteration
produced a binary or performance result.

## Results

Rustfmt, the direct representation regression, all 21 focused KBO tests, and
strict all-target/all-feature pedantic Clippy pass. Parent and candidate exit
zero with empty program stderr and byte-identical LUSK6 proof output, SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Parent | `203423eda2cf` | 8,267,936 | 7,474,421,606 |
| Candidate | `6e9ce7b61ec3` | 8,268,128 | 7,458,251,580 |

Exact same-worker work falls by 16,170,026 instructions (`0.216338%`).
Against Experiment 310's matched C count of `5,254,418,333`, the instruction
ratio moves from `1.422502x` to `1.419425x`. The candidate executable grows by
192 bytes.

The intended `mfy_vwb` attribution falls from 201,048,015 to 184,764,902
instructions, down 16,283,113 (`8.0991%`), and explains essentially the
complete whole-program change. Both binaries make the same 340,049
`mfy_vwb` calls: 222,730 from the left balance path and 117,319 from the
right.

Two independent 64-pair blocks provide 128 alternating native pairs. Every
run has the exact proof hash. The candidate wins 67 wall and CPU pairs and
improves:

| Native metric | Block 1 | Block 2 | All 128 | Combined stable 64 |
| --- | ---: | ---: | ---: | ---: |
| Paired mean wall | `0.428960%` | `0.211243%` | `0.320101%` | `0.248191%` |
| Paired mean CPU | `0.430541%` | `0.210786%` | `0.320664%` | `0.249694%` |
| Paired median wall | | | `0.095958%` | `-0.109891%` |
| Paired median CPU | | | `0.101107%` | `-0.104023%` |
| Candidate wins | 34 | 33 | 67 | 31 |

The stable-half median is slightly adverse, but both independent blocks,
their stable-half paired means, the combined paired means, and deterministic
work all improve. The exact reduction is localized to the intended KBO
owner.

Fresh comprehensive run `.artifacts/linode/260726-144607-93c2/` validates the
exact candidate binary SHA-256
`6e9ce7b61ec3abb6c9d3a4af9a164e628adbcbaf6f4bd8493a647ca9399fce8a`:

- 4,418 Rust tests across 33 result groups, Rustfmt, strict pedantic Clippy,
  and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean pinned first-order and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior;
- smoke Callgrind records 9,999,762 Rust versus 7,590,630 C instructions; and
- the ten-case aggregate improves from Experiment 325's `1.1119780745x` to
  `1.1081607572x` C, a `0.3433%` relative improvement.

`VALIDATION_COMPLETE` and `SUCCESS` both contain `ok`. Both measured workers
and firewalls were deleted after artifact collection.

## Decision

Accept. The candidate preserves every argument representation, optional-slot
behavior, push order, dereference mode, owner lifetime, proof,
compatibility, portability, and resource result. It reduces first-order KBO
balance work 8.10%, whole-program exact work 0.216%, and native paired means
in both independent blocks and their stable halves. The maintained aggregate
remains above `1.10x`, so main-prover performance parity remains open.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-025-small-arity-kbo-push/experiment-326/
.artifacts/experiments/2026-07-25-025-small-arity-kbo-push/remote.tar.gz
.artifacts/linode/260726-144607-93c2/
```

The focused archive SHA-256 is
`F823A53E26177EB359A4F89D9A9E04C069422797BDF2DB83170F8395DCD8EFC3`.

## Limits

- The direct inline arms deliberately preserve `flatten`'s behavior by
  conditionally pushing each optional slot rather than assuming
  initialization.
- The private `TermArgs` constructor remains the sole owner of the
  zero/one/two/heap shape invariant; no layout or public API changes.
- The combined stable-half medians are adverse by about 0.10%, although both
  stable paired means improve about 0.25%. The accept decision therefore
  relies on independent positive means plus the localized deterministic
  instruction reduction, not median timing alone.
- The accepted parent executable SHA-256 is the exact Experiment 325 binary.
  Its instruction count was 7,474,534,715 on that earlier worker and
  7,474,421,606 here, a 113,109-instruction (`0.00151%`) cross-worker/runtime
  variation. The candidate decision uses the same-worker parent/candidate
  delta.
- The comprehensive aggregate is load-sensitive across fresh workers. Its
  direction agrees with retained exact and paired evidence, but it is context
  rather than the causal acceptance measurement.
