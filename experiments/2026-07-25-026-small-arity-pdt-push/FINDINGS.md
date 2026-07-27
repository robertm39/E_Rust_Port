# Experiment 327: Small-arity PD-tree argument push

## Status

Complete and rejected for Bead `E_Rust_Port-j76.5.5`. Accepted production is
unchanged at Experiment 326 commit `07133485`.

## Question

Can PD-tree matching's borrowed first-order symbol expansion push zero-, one-,
and two-argument `TermArgs` shapes directly in reverse, retaining exact
uninitialized-slot diagnostics and the heap fallback while removing generic
slice, enumeration, and reverse-iterator work from the hot search owner?

## Baseline

- Accepted production: Experiment 326 commit `07133485`.
- Matched accepted LUSK6 work: `7,458,138,475` instructions on this worker.
- `BorrowedTermCell::push_initialized_arguments_reversed`, used only by the
  borrowed PD-tree query cursor, maps every `TermArgs` shape through
  `as_slice().iter().enumerate().rev()`.
- Latest comprehensive aggregate: `1.1081607572x` C.

## Candidate

The helper matches directly on private `TermArgs`. `Empty` returns zero,
`One` pushes slot zero and returns one, `Two` pushes slots one then zero and
returns two, and `Heap` retains the existing enumerated reverse traversal.
Every uninitialized slot keeps the exact
`term argument {index} is uninitialized` invariant diagnostic.

A direct regression keeps owners live while verifying returned arity and
reverse stack order across arities zero through four. The full PD-tree test
module preserves first-order stack order, parked owners, query/backtracking
state, repeated-variable behavior, pruning, insertion/deletion, and storage
accounting.

## Setup and exact commands

Focused measurement used dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-151552-6352` with Rust 1.97.1 and Callgrind 3.22.0. The
uploaded snapshot SHA-256 was
`0d9d11c88f9eac6f763c1cc6977428a3a71ae3602be6c84b07350c55da91819e`;
the accepted parent archive SHA-256 was
`035426470D5AEB7A43F0D6CCFAACF4DDD0918D495808AF0438F417D82D0E89B0`.
The candidate source hash was:

```text
351899216ef5d2b59d98a7a568aade085bfce3a280b6e07ada4cbbd34d8e885d  src/terms/termtypes.rs
```

The lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar 07133485 `
  src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-026-small-arity-pdt-push/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-327
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-026-small-arity-pdt-push/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-327
    .\.venv\Scripts\python.exe -c `
      "import sys; sys.path.insert(0, r'tools/linode-runner'); import linode_runner as lr; state=lr.load_current(); state['remote_artifact_path']='/opt/e-rust-port/artifacts/experiment-327'; lr.save_current(state); print(lr.collect_artifacts(state))"
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

## Falsification criteria

- Empty, unary, binary, and heap-backed representations must preserve returned
  arity, reverse push order, and indexed uninitialized-slot diagnostics.
- Borrowed PD-tree owner parking, matching order, substitution backtracking,
  pruning, insertion/deletion, and cursor cleanup must remain exact.
- Exact proof work must improve and proof/status/stderr must remain exact
  before native alternating timing.
- Native timing is authoritative; both independent blocks and the combined
  stable half must show a positive paired-mean direction.

## Results

Rustfmt, the direct representation regression, all 44 focused PD-tree tests,
and strict all-target/all-feature pedantic Clippy pass. Parent and candidate
exit zero with empty program stderr and byte-identical LUSK6 proof output,
SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Parent | `6e9ce7b61ec3` | 8,268,128 | 7,458,138,475 |
| Candidate | `c8ffe1cc8fe9` | 8,268,560 | 7,440,695,097 |

Exact same-worker work falls by 17,443,378 instructions (`0.233884%`).
Against Experiment 310's matched C count of `5,254,418,333`, the instruction
ratio would move from `1.419403x` to `1.416083x`. The candidate executable
grows by 432 bytes.

The intended
`PdTree::search_next_matching_occurrence_impl::<true>` attribution falls from
1,514,251,396 to 1,496,695,090 instructions, down 17,556,306 (`1.1594%`),
and explains essentially the whole-program reduction. Both binaries retain
the same 783,453 borrowed matching-search calls and all child call counts.

Native timing rejects the candidate. Two independent 64-pair blocks provide
128 alternating pairs, every one with the exact proof hash:

| Native regression | Block 1 | Block 2 | All 128 | Combined stable 64 |
| --- | ---: | ---: | ---: | ---: |
| Paired mean wall | `0.592170%` | `0.026485%` | `0.309327%` | `0.041644%` |
| Paired mean CPU | `0.591944%` | `0.029420%` | `0.310682%` | `0.042329%` |
| Paired median wall | | | `0.356232%` | `0.062222%` |
| Paired median CPU | | | `0.349746%` | `0.061098%` |
| Candidate wins | 26 | 28 | 54 | 32 |

Every retained aggregate is adverse. The second block is close to flat, but
its whole-block and stable-half paired means still regress. Comprehensive
validation was skipped after the authoritative native gate rejected the
candidate. The candidate source and test were removed; production matches
`07133485` exactly.

## Decision

Reject. The specialization is behavior-exact and removes 17.44 million
deterministic instructions from the intended owner, but it enlarges the
binary and slows native execution in both independent blocks, the combined
sample, and the combined stable half. Instruction count is diagnostic;
native time is the performance contract. Keep the generic initialized
argument traversal.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-026-small-arity-pdt-push/experiment-327/
.artifacts/experiments/2026-07-25-026-small-arity-pdt-push/remote.tar.gz
```

The focused archive SHA-256 is
`6A5DA108229D57EC6F83DA5CC9722D36ED859C17B4A395701639BA06A53DDC5E`.

## Limits

- LUSK6 heavily exercises the borrowed PD-tree matching cursor, so this is a
  targeted hot-path rejection rather than a claim about every workload.
- Callgrind confirms reduced retired work but does not model the native
  instruction-cache, branch, and code-layout costs that dominate the decision.
- The accepted parent executable is the exact Experiment 326 binary. Its
  instruction count was 7,458,251,580 on that earlier worker and
  7,458,138,475 here, a 113,105-instruction (`0.00152%`) cross-worker/runtime
  variation. The rejection uses the same-worker parent/candidate delta and
  paired timing.
