# Experiment 317: Borrowed shared structural-weight comparison

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the structural-weight comparator match C's direct shared-term pointer
walk, without changing the safe unshared fallback, comparator decisions, type
ordering, or shared-term ownership?

## Baseline

- Accepted parent: commit `9fe7ae1e`.
- The freshly rebuilt parent retires `7,846,988,228` instructions on the
  matched LUSK6 profile.
- Matched C retires `5,254,418,333` instructions.
- The recursive parent comparator acquires `Ref` argument guards and clones
  reference-counted type handles. Its two specialized recursive copies expose
  `577,104,006` and `287,718,581` caller-inclusive instructions. Outlined
  type comparison, argument borrowing, and arity access retire another
  `76,287,607`, `56,190,374`, and `22,128,743` instructions.
- At the stable `Eqn::struct_weight_compare` owner, the term comparator costs
  `339,399,079` instructions for `500,464` calls.

## Candidate

`term_struct_weight_compare` now dispatches to a private non-owning cursor
only when both roots are shared. The cursor reads stable term-cell function
codes, cached weights, shared/DB properties, arity/arguments, and type handles
directly while recursing. This removes per-level argument `Ref` guards, type
handle clone/drop traffic, and the safe handle accessor layer.

The existing safe owned comparator remains the complete fallback for any
unshared root. It also serves as the focused equivalence oracle.

The raw path has a deliberately narrow contract:

- both roots and every descendant are shared and remain owned for the call;
- completed shared-term structure, cached metadata, and type metadata are
  immutable during comparison;
- the synchronous comparator invokes no callbacks or user code;
- `Term` is `Rc`/`RefCell` based and therefore not sent across threads; and
- the only API that can return a mutable argument guard now rejects shared
  terms with an unconditional assertion.

Ordinary single-slot construction setters remain synchronous and cannot
overlap a cursor in one thread. Shared argument `Ref` guards may overlap the
raw reads because both are read-only. Type and argument mutation are absent
from every production comparison caller.

The comparator preserves `$true` minimality, cached-weight ordering, free and
DB-variable type ordering through `TypesCmp`, arity ordering, uninitialized
slot behavior, and recursive lexicographic order. Focused tests compare the
shared cursor with the retained safe implementation across these boundaries
and pin the shared mutable-argument rejection.

## Setup and exact commands

Focused validation and measurement used dedicated worker
`e-rust-codex-260726-062316-d00b` with Rust 1.97.1 and Valgrind 3.22.0. The
final uploaded worktree snapshot SHA-256 was
`54d0bffcfaa52715660c831b596f8c543b7ca472f73dbd6da39debcac0c3d92f`.
The accepted parent archive SHA-256 was
`8C186A7B8CA359A727982EA58BB96F114D644CC6AFD1ED425CA98E605D9C2727`.
Measured candidate production-file SHA-256 values were:

```text
14d04b798f2bce543d28d64d45b0afad80163faae2b3047cdfb71d4260e8bcb3  src/terms/termfunc.rs
3f106806172906d244c37f51e239bc09b4ca1848b450a6d09d14427868f13c0d  src/terms/termtypes.rs
```

After focused measurement, one test-only mutable-guard regression changed
the final `termtypes.rs` source SHA-256 to
`6295267045107C7E69E5D78ED62E404A77C1015B0795B3E20E695D2B9D36BABC`.
It is excluded from release compilation: the comprehensive release executable
has the same SHA-256 as the focused candidate.

The focused scripts preserve the exact Rustfmt, 223 focused tests, strict
all-feature library pedantic Clippy, parent/candidate release builds,
Callgrind commands, proof comparisons, and two independent 64-pair native
commands. The controller lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar 9fe7ae1e `
  src/terms/termfunc.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-016-borrowed-struct-weight-compare/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-317
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-016-borrowed-struct-weight-compare/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-317
}
finally {
    .\linode-runner.ps1 down
}
```

Exact-source comprehensive validation used fresh worker
`e-rust-codex-260726-064514-50a4` and snapshot
`2958234f358d5871ef713cba7db4c3fc96ba26b69941d352769994c799a76f54`:

```powershell
.\linode-runner.ps1 run
```

Both successful workers and firewalls were deleted after artifact collection.

## Falsification criteria

- Shared and unshared terms must retain the existing normalized comparison
  results across `$true`, weight, free-variable type, DB-variable type, arity,
  recursive child, and uninitialized-slot boundaries.
- The raw path may run only while both roots and every structural descendant
  remain shared, live, and structurally/type immutable.
- No argument or link `RefMut` may overlap the direct reads.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact work must improve materially at the intended comparator owner, and
  repeated alternating native timing must confirm the production direction.
- The complete compatibility, resource, quality, and portability lifecycle
  must remain green.

## Results

Focused Rustfmt, all 47 term-function tests, all 18 term-cell tests, all 125
term-bank tests, all 33 rewrite tests, and strict all-feature library pedantic
Clippy pass: 223 focused tests in total. The final comprehensive run includes
the additional shared mutable-argument rejection regression.

Parent and candidate produce byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind instructions fall from `7,846,988,228` to
`7,731,396,395`, a reduction of `115,591,833` (`1.473073%`). Relative to the
matched C count, the candidate ratio is `1.471409x`. The release executable
grows by 1,640 bytes, from 8,273,248 to 8,274,888 bytes.

The intended owner explains the global result. The comparator cost visible
from `Eqn::struct_weight_compare` falls from `339,399,079` to `211,643,010`
instructions at the same `500,464` calls, a reduction of `127,756,069`
(`37.6418%`). The difference exceeds the whole-program reduction by
`12,164,236` instructions because whole-program LTO moves small costs among
unrelated owners. Recursive call counts remain identical at `2,037,807` and
`508,375`.

Two independent native blocks provide 128 alternating LUSK6 pairs. The
candidate wins 107 pairs, and every run has the exact proof hash. Across all
pairs:

- wall mean, median, paired mean, and paired median improve by `1.040863%`,
  `0.989465%`, `1.031401%`, and `0.995524%`;
- CPU mean, median, paired mean, and paired median improve by `1.040663%`,
  `1.006596%`, `1.031227%`, and `0.995225%`.

Restricting both blocks to their final halves yields 64 pairs and 52 wins:

- wall mean, median, paired mean, and paired median improve by `0.977102%`,
  `0.973711%`, `0.969011%`, and `0.942118%`;
- CPU mean, median, paired mean, and paired median improve by `0.977034%`,
  `0.983132%`, `0.968939%`, and `0.934247%`.

Raw focused evidence is under:

```text
.artifacts/experiments/2026-07-25-016-borrowed-struct-weight-compare/experiment-317/
```

Fresh comprehensive run `.artifacts/linode/260726-064514-50a4/` validates
the exact candidate executable SHA-256
`0be3d427d3d9b4a80a5d43b85bc48f3bdfbd743c9ec4ebf76419bd163635cee3`:

- 4,412 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order pinned-C references build and pass
  smoke checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior; and
- smoke Callgrind records `9,802,797` Rust versus `7,590,630` C instructions.

The fresh aggregate is `1.1332152692x` Rust/C wall time. Experiment 316's
fresh aggregate was `1.1329564938x`; this small cross-worker difference is not
used as causal evidence. The same-worker deterministic and alternating
measurements establish the candidate's direction. `VALIDATION_COMPLETE` and
`SUCCESS` both contain `ok`.

## Falsification checks and limits

- The raw cursor and type comparator are private and cannot escape through a
  public API. The safe public comparator performs the complete lifetime and
  shared-root dispatch.
- Shared term roots retain all descendants. The term bank completes cached
  structural metadata and types before setting the shared property.
- `arguments_mut` is crate-private and now rejects shared terms
  unconditionally. Shared `Ref` argument borrows may overlap the cursor because
  neither path mutates. Type/link setters do not return retained mutable guards.
- `Rc`/`RefCell` terms are single-threaded. The comparator invokes no callback,
  mutation, allocation release, or other operation that could re-enter a
  setter while raw references are live.
- The first focused source snapshot failed before measurement because the test
  module did not import the retained private comparison oracle. The second
  failed its new focused test because manually constructed shared fixtures had
  cached weight but not matching variable/function counts. The final fixture
  initializes all shared metadata; neither failed snapshot contributed
  performance evidence.
- The first comprehensive controller invocation was intentionally terminated
  by a one-second local command timeout while the Linode was still
  provisioning. Managed Linode `101418065` and firewall `88421038` were
  immediately deleted; no source was uploaded and it contributed no
  validation evidence.
- The aggregate remains above `1.10x`; this closes one measured
  reference-count/borrow differential, not the performance epic.

## Decision

Accept. The unsafe scope is private, documented, and justified by a measured
C-shaped ownership gap. Focused semantic tests cover comparator and mutation
boundaries, the intended owner falls 37.64%, exact whole-program work falls
1.47%, both independent native blocks improve, and the complete compatibility,
resource, portability, and quality matrices remain green. Main-prover
performance parity remains open.
