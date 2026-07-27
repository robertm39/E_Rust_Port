# Experiment 315: Buffer term-tree splay chains

## Status

Complete and rejected for Bead `E_Rust_Port-j76.5.5`. Production is
unchanged.

## Question

Can the term-cell store reuse two safe `Vec<Term>` scratch chains during
top-down splaying, moving traversed nodes into those chains and rebuilding the
intrusive links at exit, instead of cloning one extra reference-counted tail
handle for every chain extension?

## Baseline

- Accepted parent: commit `968f00d3`, whose production source is identical to
  accepted performance commit `dd0575f4`.
- The paired rebuild in this experiment executes `7,972,417,882` Rust
  instructions on LUSK6. The matched C count remains `5,254,418,333`, so the
  accepted implementation is `1.517279x` C in this profile.
- The accepted visible `term_top_insert` boundary executes `1,081,505,063`
  instructions in this paired rebuild.
- C keeps raw stack-local left/right tail pointers. Experiment 236 rejected an
  unsafe non-owning Rust tail because it raised both insertion and
  whole-program instructions. It did not test owned move-only chains or reuse
  scratch storage across the complete term-cell store.

## Candidate

- Add one reusable `TermSplayScratch` to `TermCellStore`, containing separate
  left and right `Vec<Term>` chains shared by every one of its 32,768 buckets.
- During each top-down splay, move each traversed node into the appropriate
  chain instead of cloning a second owning tail handle.
- Rebuild the intrusive links by popping each chain after the search reaches
  its new root.
- Keep standalone `TermTree` callers working through the existing public
  operations with a temporary scratch value.
- Add a 128-node zig-zag regression that finds every node, extracts a patterned
  subset, and checks the remaining node count.

## Falsification criteria

- Splay ordering, rotations, root selection, duplicate handling, extraction,
  and deletion must remain unchanged.
- Every buffered node must be moved back into exactly one owning tree link
  before the splay returns.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact work must improve at the intended insertion/splay boundary.
- Alternating native measurements must confirm that vector bookkeeping and
  locality do not reverse the instrumented result.

## Reproduction

- Dedicated worker:
  `e-rust-codex-260726-045012-a018` (Linode `101412370`, firewall
  `88281723`; both deleted after artifact download).
- Worker toolchain: Rust `1.97.1`.
- Uploaded source archive SHA-256:
  `8385c7325d3784979e7cd449e2d4ae37cc0a48832653e01c1013c27007ab5f6d`.
- Candidate source SHA-256:
  - `src/terms/termcellstore.rs`:
    `b6e5f5fdddc40f620f9914817bc1bc5497a60e38348663e3d2cdf1e56e47c365`
  - `src/terms/termtrees.rs`:
    `3e7fdd5594c3af758d37805996ffd2cacab07bbf11f4d854e0249290815e6903`
- [`remote_measure.sh`](remote_measure.sh) contains the exact formatting,
  focused-test, strict-Clippy, build, Callgrind, proof-comparison, and
  alternating native-measurement commands. [`remote_repeat.sh`](remote_repeat.sh)
  was prepared before measurement but deliberately not run after the first
  native block falsified the candidate.

## Results

### Validation

- `cargo fmt --all -- --check` passed.
- All 6 focused term-tree tests passed, including the new zig-zag regression.
- All 7 term-cell-store tests passed.
- All 125 term-bank tests passed.
- Strict all-feature library pedantic Clippy passed.
- Parent and candidate produced byte-identical proof output with SHA-256
  `b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`;
  both stderr files were empty.

### Exact work

| Measurement | Parent | Candidate | Candidate delta |
| --- | ---: | ---: | ---: |
| Whole LUSK6 | 7,972,417,882 | 7,937,641,938 | -34,775,944 (-0.436203%) |
| `term_top_insert` self | 1,081,505,063 | 1,046,819,173 | -34,685,890 (-3.207187%) |

The intended insertion owner accounts for 99.7410% of the whole-program
instruction reduction. Candidate work is `1.510660x` the matched C count.

The parent release executable is 8,271,984 bytes with SHA-256
`fd0cba1276a53ef930bd8c2b4c6e2de11809bfe6904ad7d87f6ed482db7afd79`.
The candidate is 8,270,944 bytes (-1,040) with SHA-256
`8c70df7bf2c475a77abe188088b06de572cf0451f843dd0d0d4b2d2c1909f67a`.

### Native timing

The complete 64-pair alternating block rejects the candidate:

- 29 candidate wins out of 64.
- Mean wall time regresses 0.244886%; paired mean regresses 0.253898%.
- Median wall time regresses 0.290182%; paired median regresses 0.219184%.
- Mean CPU time regresses 0.243430%; paired mean regresses 0.252395%.
- Median CPU time regresses 0.282999%; paired median regresses 0.217401%.

The final 32 pairs show the same direction rather than a warmup artifact:

- 15 candidate wins out of 32.
- Mean wall time regresses 0.288547%; paired mean regresses 0.295867%.
- Median wall time regresses 0.262156%; paired median regresses 0.123424%.
- Mean CPU time regresses 0.284478%; paired mean regresses 0.291789%.
- Median CPU time regresses 0.260277%; paired median regresses 0.111462%.

Because both the complete block and its less warmup-sensitive final half
regress consistently, a second block and comprehensive validation were
deliberately skipped.

## Decision

Reject and restore the accepted direct-link splay exactly. Buffering the chains
removes reference-count traffic and is a clean instruction-count win, but
`Vec` push/pop bookkeeping and the delayed reverse-order link rebuild worsen
the production executable's locality enough to reverse that win in native
timing. The candidate fails the explicit native falsification criterion.

Raw evidence is retained under
`.artifacts/experiments/2026-07-25-014-buffered-term-splay-chains/experiment-315/`.
