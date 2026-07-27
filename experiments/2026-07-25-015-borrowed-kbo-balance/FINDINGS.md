# Experiment 316: Borrowed KBO balance traversal

## Status

Complete and accepted for Bead `E_Rust_Port-j76.5.5`. The broader performance
target remains open.

## Question

Can the first-order KBO6 variable/weight-balance walker reuse a private stack
of non-owning term-cell cursors, matching C's local tagged-pointer stack and
avoiding one `Rc` clone/drop plus owning dereference per structural edge?

## Baseline

- Accepted parent: commit `c616307b`.
- The freshly rebuilt parent retires `7,972,417,868` instructions on the
  matched LUSK6 profile.
- Matched C from Experiment 310 retires `5,254,418,333` instructions.
- The first-order `mfy_vwb` walker has `190,326,332` self instructions and
  calls `term_deref` for another `84,255,061` instructions.
- Its two caller-visible boundaries total `326,586,266` instructions:
  `246,367,494` from `kbo_lin_cmp` and `80,218,772` from its second
  specialized copy.
- C's corresponding balance walkers traverse borrowed term pointers on a
  local tagged stack. Experiment 226 already retained the Rust traversal
  vector's capacity, and Experiment 258 rejected a Boolean-side
  specialization, but neither removed reference-count ownership from each
  stored frame.

## Candidate

The first-order walker now retains a reusable
`Vec<(BorrowedTermCell, DerefType)>` in the ordering control block. Each cursor
preserves the stable address returned by `Rc::as_ptr`; following a binding and
pushing initialized arguments do not acquire or release shared ownership.
Variable balance updates consume the cursor's function code directly.

The safe comparison entry point contains the complete unsafe traversal:

- the borrowed input root owns every structural descendant until return;
- every followed binding remains owned by its variable cell;
- first-order comparison does not replace arguments or bindings, release a
  reachable root, or invoke user code;
- all cursors preserve `Rc::as_ptr` provenance, alignment, initialization,
  and allocation identity; and
- first-order dispatch cannot encounter applied-variable expansion.

The scratch stack is cleared before its first cursor dereference. A caught
invariant panic may leave dangling pointer values in the vector, but the next
entry discards those values without dereferencing or dropping them. A focused
catch-and-reuse regression forces such an unwind while another cursor remains,
then safely reuses the same ordering control block.

The existing owning `Vec<(Term, DerefType)>` and all LFHO/Lambda walkers remain
unchanged. The cursor methods, stack, and unsafe operations are crate-private
and carry explicit safety contracts.

## Setup and exact commands

Focused validation and measurement used dedicated worker
`e-rust-codex-260726-051654-405d` with Rust 1.97.1. The final uploaded
worktree snapshot SHA-256 was
`4a91cb65539f979bca9cb52205f3c030f6abe177500f8e6bda6bb2c3678c186a`.
The accepted parent archive SHA-256 was
`A2B2B5B2D7E1EB147C5A83CCAD204DB7A9D9919CAB2BCD5BCA0188993449C03E`.
Candidate production-file SHA-256 values were:

```text
78a8e7172ba2a7292c541f9d19aa7ef7b28b260672427f25ad719cc1fe1ec83a  src/orderings/cto_kbolin.rs
5fe44d165b69d80adbc544c3ef1a5f4236306ee33dd0d262872ee269def637c7  src/orderings/ocb.rs
22b52cb6428956027302dea183e3a1f81d91aa99271c529f55bd9629b5973714  src/terms/termtypes.rs
```

The focused scripts preserve the exact Rustfmt, 59 focused tests, strict
all-feature library pedantic Clippy, parent/candidate release builds,
Callgrind commands, proof comparisons, and two independent 64-pair native
commands. The controller lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar c616307b `
  src/orderings/cto_kbolin.rs src/orderings/ocb.rs src/terms/termtypes.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-015-borrowed-kbo-balance/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-316
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-015-borrowed-kbo-balance/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-316
}
finally {
    .\linode-runner.ps1 down
}
```

Exact-source comprehensive validation used fresh worker
`e-rust-codex-260726-054229-f44b` and snapshot
`ef6999b0240739c2f3eec6c0bdf1deb2911b31604865c8a9c73b216f46e1282c`:

```powershell
.\linode-runner.ps1 run
```

Both successful workers and firewalls were deleted after artifact collection.

## Falsification criteria

- First-order dereference mode, LIFO argument order, function/variable
  weights, positive/negative balances, and comparison results must remain
  unchanged.
- A borrowed cursor may be dereferenced only while the caller's root and every
  followed binding owner remain live and structurally unchanged.
- The existing owned LFHO and Lambda-order walkers must remain unchanged.
- A caught invariant panic must not cause a later traversal to dereference a
  stale cursor.
- Parent and candidate must produce byte-identical LUSK6 proof output.
- Exact work must improve at the intended KBO balance owner, and alternating
  native timing must confirm the production direction.
- The complete compatibility, resource, quality, and portability lifecycle
  must remain green.

## Results

Focused Rustfmt, all 21 KBO tests, all 20 ordering-control tests, all 18
term-cell tests, and strict all-feature library pedantic Clippy pass. This
includes the caught-panic stale-cursor regression.

Parent and candidate produce byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind instructions fall from `7,972,417,868` to
`7,846,989,261`, a reduction of `125,428,607` (`1.573282%`). Relative to the
matched C count, the candidate ratio is `1.493408x`. The release executable
grows by 1,264 bytes, from 8,271,984 to 8,273,248 bytes.

The intended owner explains the global result. The two caller-visible
`mfy_vwb` boundaries fall from `326,586,266` to `201,048,253` instructions,
a reduction of `125,538,013` (`38.4395%`) that accounts for `100.0872%` of
the whole-program reduction. The candidate's complete first-order walker
collapses into `201,048,015` self instructions and no longer calls
`term_deref`; small unrelated LTO movements explain the 0.0872% excess over
the global reduction.

Two independent native blocks provide 128 alternating LUSK6 pairs. The
candidate wins 90 pairs, and every run has the exact proof hash. Across all
pairs:

- wall mean, median, paired mean, and paired median improve by `0.756815%`,
  `0.825354%`, `0.736316%`, and `0.653423%`;
- CPU mean, median, paired mean, and paired median improve by `0.759104%`,
  `0.826581%`, `0.738633%`, and `0.656174%`.

Restricting both blocks to their final halves yields 64 pairs and 44 wins:

- wall mean, median, paired mean, and paired median improve by `1.147172%`,
  `0.820059%`, `1.113097%`, and `0.675771%`;
- CPU mean, median, paired mean, and paired median improve by `1.147268%`,
  `0.819175%`, `1.113226%`, and `0.683581%`.

Raw focused evidence is under:

```text
.artifacts/experiments/2026-07-25-015-borrowed-kbo-balance/experiment-316/
```

Fresh comprehensive run `.artifacts/linode/260726-054229-f44b/` validates
the exact candidate executable SHA-256
`9d2053288a53332753e7463cdbd429f8daf3482e19ea290d043491c67eb41203`:

- 4,411 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order pinned-C references build and pass
  smoke checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior; and
- smoke Callgrind records `9,802,513` Rust versus `7,590,630` C instructions.

The fresh aggregate is `1.1329564938x` Rust/C wall time, with LUSK6 at
`1.374436x` and LUSK6ext at `1.366925x`. Experiment 313's fresh aggregate was
`1.1332926602x`; this small cross-worker difference is not used as causal
evidence. The same-worker deterministic and alternating measurements establish
the candidate's direction. `VALIDATION_COMPLETE` and `SUCCESS` both contain
`ok`.

## Falsification checks and limits

- The raw cursor is private to term/order traversal and cannot escape through
  a public API.
- The live input root owns every structural argument allocation; active
  variable bindings own every followed target. KBO comparison must never
  replace argument or binding slots or release roots while a cursor is
  pending.
- Applied-variable expansion is excluded by the first-order dispatcher.
  Higher-order balance walkers retain owned `Term` handles and the existing
  dereference helpers.
- A caught panic may leave stale pointer bits in the reusable vector, but
  entry clears them before any pointer dereference. Raw pointers have no drop
  glue, so clearing cannot access the expired allocation.
- The first provisioning attempt
  (`e-rust-codex-260726-051511-c518`) timed out locally before bootstrap and
  contributed no evidence. The successful focused worker's first sync stopped
  at Rustfmt; the final formatted snapshot was resynchronized before any
  parent/candidate build or measurement.
- The first comprehensive attempt
  (`e-rust-codex-260726-053501-0efc`) lost its SSH connection during the
  initial Rust compile before validation completed. It contributed no test or
  performance evidence. The complete results above come only from the fresh
  successful worker.
- The aggregate remains above `1.10x`; this closes one measured
  reference-count traversal differential, not the performance epic.

## Decision

Accept. The unsafe scope is private, documented, and justified by a measured
C-shaped ownership gap. Focused semantic tests cover dereference and
liveness-sensitive boundaries, the intended owner falls 38.44%, exact
whole-program work falls 1.57%, both independent native blocks improve, and
the complete compatibility, resource, portability, and quality matrices
remain green. Main-prover performance parity remains open.
