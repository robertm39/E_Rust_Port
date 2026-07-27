# Experiment 328: Compact term metadata

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the remaining binding, rewrite-replacement, and type owners use one
private single-threaded `UnsafeCell` boundary instead of
`RefCell<TermLinks>`, preserving safe owned/copying APIs and the existing
contracted borrowed traversals while removing per-term borrow state and
shrinking every 64-bit `TermCell` from 152 to 144 bytes?

## Baseline

- Accepted production: Experiment 326 implementation at commit `5d0b5506`
  (Experiment 327 changed only durable experiment records).
- Matched accepted LUSK6 work: `7,458,138,485` instructions on this worker.
- `TermLinks` stores three pointer-sized optional owners, but
  `RefCell<TermLinks>` occupies 32 bytes because every term also carries the
  dynamic borrow flag.
- Experiments 313 and 314 showed that safe per-owner `Cell<Option<Term>>`
  splits either preserve the 152-byte node or reverse in native timing. No
  prior experiment tested a contained metadata cell whose safe APIs expose no
  references.
- Latest comprehensive aggregate: `1.1081607572x` C.

## Candidate

The three nullable owners move into 24-byte `TermLinkData` inside a private
`TermLinks(UnsafeCell<TermLinkData>)`. The safe surface remains:

- `binding` and `rw_replace_field` clone the selected `Term` owner;
- `type_` clones its `Type` owner and `type_uid` copies only the UID;
- setters replace one owner without returning a reference;
- unique top-cell reset uses `UnsafeCell::get_mut`; and
- `Debug` is opaque and never touches metadata.

`Term` is backed by `Rc`, so it cannot cross threads. Safe reads clone or copy
their result before returning and cannot overlap a safe setter because they
expose no reference. The private unsafe `shared` accessor is used only by six
already-borrowed dereference/type-comparison sites. Each caller owns or parks
the complete graph and explicitly forbids binding/type mutation until its
reference is dead.

The fixed two-link `deref_always_step` window retains its accepted
clone-at-exit behavior with two contracted metadata reads instead of
`RefCell::borrow`. All unsafe operations have adjacent safety comments, and
the cell accessor documents allocation liveness, single-threaded access, and
no-overlapping-mutation obligations.

The 64-bit layout regression pins 24-byte `TermLinkData` and `TermLinks`, a
144-byte `TermCell`, opaque debug behavior, all five owner/link APIs, and
replacement lifetimes. A new lifetime regression proves safe binding and
rewrite getters clone their owners before a setter drops the installed owner.

## Setup and exact commands

Focused measurement used dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-153857-456c` with Rust 1.97.1 and Callgrind 3.22.0. The
accepted parent archive SHA-256 was
`FEBBAE2780D66597E3269EC091C1D004DAEE61CF76C43E2E3C81EDAED30009B8`.
The exact candidate source hash was:

```text
49ee7b7c7cf3b7a7f038d7ed19c70e7423e28c04cf6b39de61cb3d75e2b0c163  src/terms/termtypes.rs
```

The focused lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar 5d0b5506 `
  src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-027-compact-term-metadata/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-328
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-027-compact-term-metadata/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-328
    .\.venv\Scripts\python.exe -c `
      "import sys; sys.path.insert(0, r'tools/linode-runner'); import linode_runner as lr; state=lr.load_current(); state['remote_artifact_path']='/opt/e-rust-port/artifacts/experiment-328'; lr.save_current(state); print(lr.collect_artifacts(state))"
}
finally {
    .\linode-runner.ps1 down
}
```

The focused gates ran all term-type, substitution, term-store, term-tree,
term-bank, KBO6, and clause-rewrite test modules before strict Clippy.

Both exact profiles used:

```bash
valgrind --tool=callgrind \
  BINARY eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

Fresh exact-source comprehensive validation used worker
`e-rust-codex-260726-155517-0f09`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- `TermLinks` must remain one three-pointer-wide cell and `TermCell` must fall
  from 152 to 144 bytes on 64-bit targets.
- No safe API may expose a reference into metadata. Getter clones must keep an
  owner live after replacement; setters and unique reset must drop old owners
  exactly once.
- Every unsafe shared metadata read must retain the complete owner graph and
  prohibit mutation for the reference lifetime.
- Binding chains, substitution/backtracking, rewrite chains, KBO traversal,
  term-tree/store ownership, term-bank sharing, caught-panic cleanup, and
  type comparisons must remain exact.
- Parent/candidate proof output and program stderr must be byte-identical.
- Exact work and native timing must improve in both independent blocks and
  their stable halves.
- The complete Linux/Windows/C build, test, compatibility, resource, and
  benchmark lifecycle must pass before acceptance.

## Results

Rustfmt, strict all-target/all-feature pedantic Clippy, and all focused modules
pass:

- 20 term-type tests;
- 13 substitution tests;
- seven term-store tests;
- five term-tree tests;
- 125 term-bank tests;
- 21 KBO6 tests; and
- 33 clause-rewrite tests.

Parent and candidate exit zero with empty program stderr and byte-identical
LUSK6 proof output, SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Parent | `6e9ce7b61ec3` | 8,268,128 | 7,458,138,485 |
| Candidate | `64f3f47532a1` | 8,236,584 | 7,139,309,315 |

Exact same-worker work falls by 318,829,170 instructions (`4.274916%`).
Against Experiment 310's matched C count of `5,254,418,333`, the instruction
ratio moves from `1.419403x` to `1.358725x`. The candidate executable shrinks
by 31,544 bytes.

The reduction is distributed across the intended metadata-heavy owners:

| Owner | Parent | Candidate | Reduction |
| --- | ---: | ---: | ---: |
| Substitution normalization self | 310,162,887 | 257,873,551 | `16.8587%` |
| Rewrite-chain self | 118,116,000 | 103,305,465 | `12.5390%` |
| KBO variable balance self | 184,764,902 | 171,894,026 | `6.9661%` |
| Term-top insertion self | 691,363,995 | 662,741,511 | `4.1400%` |
| Borrowed PD-tree matching self | 1,514,251,396 | 1,476,861,982 | `2.4692%` |

The layout also changes allocation addresses, so the term-top comparison
count moves from 7,113,427 to 7,128,737 while proof output remains exact.
Pointer-order topology is process-local in both the C implementation and the
port; the decision relies on output/resource compatibility rather than
requiring identical allocator addresses.

Two independent 64-pair blocks provide 128 alternating native pairs. Every
run has the exact proof hash. The candidate wins 127 wall and CPU pairs and
all 64 combined stable-half pairs:

| Native improvement | Block 1 | Block 2 | All 128 | Combined stable 64 |
| --- | ---: | ---: | ---: | ---: |
| Paired mean wall | `3.946699%` | `3.892994%` | `3.919846%` | `3.772128%` |
| Paired mean CPU | `3.949732%` | `3.892134%` | `3.920933%` | `3.773608%` |
| Paired median wall | | | `3.898670%` | `3.935887%` |
| Paired median CPU | | | `3.891021%` | `3.934725%` |
| Candidate wins | 64 | 63 | 127 | 64 |

Fresh comprehensive run `.artifacts/linode/260726-155517-0f09/` validates the
exact focused candidate binary SHA-256
`64f3f47532a14bd154f08eba75e8f7505a9971c2956dec8c3e28f7937fb9a239`:

- 4,419 Rust tests across 33 result groups, Rustfmt, strict pedantic Clippy,
  and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean pinned first-order and higher-order C references build and pass smoke
  checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior;
- smoke Callgrind records 9,990,588 Rust versus 7,590,616 C instructions; and
- the ten-case aggregate improves from Experiment 326's `1.1081607572x` to
  `1.1027921183x` C, a `0.4845%` relative improvement.

Representative comprehensive peak RSS remains stable: LUSK6 falls 198,060 to
197,896 KiB, LUSK6ext falls 385,212 to 385,080 KiB, and the 1.9-GiB BOO020
resource boundary moves by only +568 KiB (`0.030%`) across fresh workers.

`VALIDATION_COMPLETE` and `SUCCESS` both contain `ok`. Both measured workers
and firewalls were deleted after artifact collection.

## Decision

Accept. This is the concrete measured-performance case permitted by the
project's unsafe policy. The candidate removes eight bytes and dynamic borrow
state from every term node while retaining safe owned/copying APIs, containing
all raw reads behind documented contracts, preserving owner drop behavior,
and passing every focused and repository-wide correctness boundary. Exact
work falls 4.27% and native LUSK6 time improves about 3.92% in independent
blocks and stable halves.

The maintained aggregate remains above `1.10x` at `1.1027921183x`, so
main-prover performance parity remains open.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-027-compact-term-metadata/experiment-328/
.artifacts/experiments/2026-07-25-027-compact-term-metadata/remote.tar.gz
.artifacts/linode/260726-155517-0f09/
```

The focused archive SHA-256 is
`0F832E909D3731AD91F21B6330869AA2C199917F39A2C8E7539C329462F066B9`.

## Limits

- `TermLinks::shared` remains an unsafe internal API. Adding any caller
  requires an owner-liveness and no-overlapping-mutation audit; safe code must
  continue using the cloned/copied accessors.
- `Term` deliberately remains `Rc`-backed and single-threaded. Replacing it
  with a cross-thread owner would invalidate the metadata-cell justification
  and require a new synchronization design.
- The opaque `Debug` representation deliberately avoids reading metadata and
  cannot display binding/rewrite/type owners.
- The comprehensive aggregate is load-sensitive across fresh workers and
  moves much less than the controlled LUSK6 result. Same-worker exact and
  paired measurements are the causal acceptance evidence.
- The accepted parent executable recorded 7,458,251,580 instructions in
  Experiment 326 and 7,458,138,485 here, a 113,095-instruction (`0.00152%`)
  cross-worker/runtime-startup variation. The decision uses the same-worker
  parent/candidate delta.
