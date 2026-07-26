# Experiment 312: Borrowed substitution-normalization cursor

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`.

## Question

Can `Substitution::norm_term` match C's local non-owning term-pointer stack
without weakening public ownership, by retaining one scoped root and traversing
stable `Rc<TermCell>` allocations through a private raw cursor?

## Baseline

- Accepted source: commit `7e4903d3`.
- Matched Experiment 311 LUSK6 profile: `8,146,788,965` Rust instructions;
  the freshly rebuilt parent in this experiment retires `8,146,683,472`.
- Matched C reference: `5,254,418,333` instructions.
- Rust `Substitution::norm_term` subtree: `552,627,762` instructions, including
  `439,641,877` self instructions.
- C `SubstNormTerm` subtree: `271,592,163` instructions, including
  `192,675,144` self instructions.
- The fresh ten-case native aggregate is `1.1326882018x` C, above the normal
  `1.10x` target.

## Candidate

`Substitution` retains a reusable vector of private `BorrowedTermCell` cursors
instead of owned `Term` handles for normalization. Each cursor is the stable
allocation pointer returned by `Rc::as_ptr`; it never changes reference counts
while walking ordinary arguments and binding chains. Only a newly freshened
source variable is converted to an owned handle, and that handle moves
directly into the substitution stack.

The safe `norm_term` entry point contains the complete unsafe traversal:

- its borrowed input root owns every structural descendant until return;
- normalization never replaces argument slots or removes a binding;
- existing and newly added bindings retain every followed target;
- no mutable `TermCell` reference exists, and ordinary `Cell`/`RefCell`
  boundaries continue to enforce interior mutation;
- each pointer preserves `Rc::as_ptr` provenance, alignment, initialization,
  and allocation identity; and
- applied-free-variable expansion uses the existing owned dereference helper
  and retains every temporary expansion root until the raw stack is empty.

An RAII scratch guard clears every pending cursor during unwinding before
temporary expansion roots are released. A regression catches a missing-type
panic while another cursor remains pending, confirms the scratch is empty,
and safely reuses the same substitution.

The cursor type and all unsafe operations remain crate-private, carry explicit
`# Safety` contracts and adjacent `SAFETY` justifications, and are locally
allowed under the crate's otherwise denied unsafe-code policy. Public term and
substitution ownership is unchanged.

The existing binding-order regression now repeats a shared variable. New tests
cover an existing binding chain and higher-order applied-variable expansion,
including exact substitution positions, target marking, ownership through
traversal, and backtracking.

## Setup and exact commands

Focused measurement used dedicated worker
`e-rust-codex-260726-015331-8ae9` with Rust 1.97.1 and final immutable source
snapshot
`f1ee432c160594ec034a874c5255d5d3d4b8a29dc25d79e2c435a7771bfc0f53`.
The accepted parent was commit `7e4903d3`; its two changed production files
were transferred in a local `git archive` so both release binaries were built
on the same worker without repository metadata:

```powershell
git archive --format=tar --output=accepted-source.tar 7e4903d3 `
  src/terms/subst.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-011-borrowed-subst-normalization/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-312
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-011-borrowed-subst-normalization/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-312
}
finally {
    .\linode-runner.ps1 down
}
.\linode-runner.ps1 run
```

The focused scripts preserve the exact remote Rustfmt, 30 focused tests,
strict all-feature library pedantic Clippy, release builds, Callgrind,
proof-identity checks, and two independent 64-pair native commands. The final
command created comprehensive worker
`e-rust-codex-260726-020931-a1ba` from snapshot
`748ba4c07bc3e1ffd3e86e9fea27fe09c3b5737fa51a6b71a0ac3a19fb18934d`.
Both workers and firewalls were deleted after artifact collection.

## Results

Focused Rustfmt, all 11 substitution tests, all 18 term-cell tests, and strict
all-feature library pedantic Clippy pass. Parent and candidate produce
byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit 0.

Matched Callgrind instructions fall from `8,146,683,472` to
`8,068,126,076`, a reduction of `78,557,396` (`0.964287%`). Relative to the
matched C count of `5,254,418,333`, the candidate ratio is `1.535494x`.
The release executable grows by 1,408 bytes, from 8,284,584 to 8,285,992
bytes.

The intended owner explains the result:

- `Substitution::norm_term` self work falls from 439,641,877 to 343,678,069
  instructions, a reduction of 95,963,808 (`21.8277%`);
- its complete subtree falls from 552,628,555 to 478,243,971 instructions, a
  reduction of 74,384,584 (`13.4601%`); and
- that subtree reduction accounts for 94.69% of the whole-program reduction,
  rather than an unrelated LTO shift.

Two independent native blocks provide 128 alternating LUSK6 pairs. The
candidate wins 115 pairs and every run has the exact proof hash. Across all
pairs:

- mean wall and CPU time improve `1.352743%` and `1.352654%`;
- paired mean wall and CPU time improve `1.347288%` and `1.347202%`;
- median wall and CPU time improve `1.302707%` and `1.302846%`; and
- paired median wall and CPU time improve `1.458847%` and `1.458000%`.

The combined final halves retain 60 wins in 64 pairs and paired mean
improvements of `1.423950%` wall and `1.424527%` CPU, ruling out a warmup-only
effect.

Fresh comprehensive run `.artifacts/linode/260726-020931-a1ba/` validates the
exact accepted source:

- 4,410 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior; and
- smoke Callgrind records 9,610,554 Rust versus 7,590,630 C instructions.

The comprehensive LUSK6 and LUSK6ext Rust/C ratios are `1.38610x` and
`1.42531x`, narrowly better than Experiment 311's `1.39076x` and `1.43864x`.
The load-sensitive aggregate is noisier at `1.1474595106x` versus the preceding
`1.1326882018x`. The same-worker parent/candidate pairs and deterministic
profile establish the candidate's direction; the normal `1.10x` target
remains open.

The lifecycle wrote `VALIDATION_COMPLETE` and `SUCCESS`, collected the complete
reports, and deleted its Linode and firewall.

Raw focused artifacts:

```text
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/callgrind-instructions.txt
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/callgrind-parent.out
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/callgrind-candidate.out
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/callgrind-parent-tree.txt
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/callgrind-candidate-tree.txt
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/native-lusk.csv
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/native-lusk-2.csv
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/native-summary.json
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/native-summary-2.json
.artifacts/experiments/2026-07-25-011-borrowed-subst-normalization/corrected/native-summary-combined.json
.artifacts/linode/260726-020931-a1ba/validation-summary.json
```

## Falsification checks and limits

- The raw cursor is private to term/substitution traversal and cannot escape
  through a public API.
- A live input root owns every structural argument allocation. Normalization
  may add variable bindings and property bits, but must never replace argument
  slots or remove bindings while a raw cursor can reference them.
- Higher-order applied-variable expansion returns owned temporary roots; those
  roots must remain live until the raw traversal stack is empty.
- Every unsafe operation requires the repository's local allowance, `SAFETY`
  comment, and explicit validity/provenance/aliasing contract.
- Focused tests must cover binding chains, repeated shared variables,
  applied-variable expansion, exact binding order, backtracking, and
  catch-and-reuse after a mid-traversal panic.
- Parent and candidate must retain byte-identical proof output and exact exit
  behavior on the matched worker.
- Deterministic work must improve substantially enough to justify unsafe Rust,
  then repeated alternating native pairs and the comprehensive lifecycle must
  independently accept the candidate.
- The first focused attempt stopped at Rustfmt before compilation. The second
  passed all focused tests but strict Clippy rejected one unused cursor method;
  that method was removed before either parent/candidate build or measurement.
  Neither stopped attempt contributes performance data.
- A first measured candidate passed focused and comprehensive gates, but the
  post-run audit found that caught unwinding could retain dangling scratch
  cursors. Its artifacts are preserved only as superseded diagnostic evidence.
  The accepted RAII-guarded source was rebuilt, reprofiled, rerun for both
  native blocks, and revalidated comprehensively; all results above are from
  that corrected source.
- The raw stack is safe only because normalization is a read-only structural
  walk with additive bindings. It must not be reused for an operation that can
  replace arguments, clear bindings, or release reachable roots.
- An RAII guard clears the borrowed scratch on ordinary return and unwinding
  before temporary expansion roots are released. A catch-and-reuse regression
  forces a missing-type panic while another raw cursor remains pending, then
  safely reuses the same substitution.
- Higher-order normalization retains the port's existing applied-variable
  dereference behavior. C's separate `WHNF_deref` selection remains an
  ownership/API compatibility item rather than part of this optimization.
- The fresh aggregate remains above `1.10x`; this closes the measured
  reference-count traversal differential, not the performance epic.

## Decision

Accept. The unsafe scope is private, documented, and justified by a measured
C-shaped ownership gap that safe variants repeatedly failed to close.
Focused semantic tests cover the liveness-sensitive boundaries, exact
normalizer work falls 13.46%, whole-program work falls 0.96%, both independent
native blocks improve by about 1.35%, and the complete compatibility,
resource, portability, and quality matrices remain green. Main-prover
performance parity remains open.
