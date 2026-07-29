# Findings

## Result

The caller-supplied external-checker path passed every preregistered semantic
gate. ProofGuard 1.0 returned `VerifiedGood` for both the minimized
definition-dependent proof and a fresh `PUZ008-2` static-splitting proof.
Umlaut's positive-only validation controller accepted both without
`--allow-coverage-gap`.

ProofCheck 1.0 returned `Unknown` on the same two proofs, preserving the
reproduced coverage boundary. The proof objects themselves were not adapted
or weakened.

## Independent checker boundary

The experiment used ProofGuard from the exact detached Git commit
`18fc573131648c9d1ed81e818f52f704c435033e`. Before execution, the controller
required:

- the exact upstream URL
  `https://github.com/ValueAchooMatthew/ATP-Research-Project.git`;
- a clean checkout;
- SHA-256
  `4da81bc5fb1651e01b2d5e5ae233b044ee20c58b8b67aa9644887cd42498471c`
  for `proover-check`; and
- SHA-256
  `1441ed3a18702a97f83d9dccd5c2ef1fd9b0832a846bba709d4260bba19e8863`
  for `proover.py`.

ProofGuard first checks that an introduced predicate is fresh, non-circular,
and parameterized by all variables used in its body. It then invokes a
separate E process to verify every theorem-preserving dependent inference.
The E 3.2.5 binary used here had SHA-256
`f850cf40c120acfa5a4af5dfcf3cd681e6fb96da63ca30dd12ab6cf66d90483d`.
ProofGuard's 21-case upstream suite passed before the experiment cases ran.

ProofCheck 1.0 retained executable SHA-256
`92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`
and self-certified all 117 bundled tests.

The pinned ProofGuard revision has no license declaration. It is therefore
not copied, linked, packaged, or downloaded by Umlaut. The supported path is
explicitly opt-in and caller-supplied. Callers must obtain any required
permission and keep the external checkout outside Umlaut's source and runtime
packages.

## Positive cases

| Case | Umlaut status | ProofGuard | ProofCheck | Gate | Checker wall |
|---|---|---|---|---|---:|
| Minimized used definition | `Unsatisfiable` fixture | `VerifiedGood` | `Unknown` | `verified` | 0.045941 s |
| `PUZ008-2` static split | `Unsatisfiable` | `VerifiedGood` | `Unknown` | `verified` | 0.156903 s |

The fresh `PUZ008-2` proof was 14,178 bytes, had SHA-256
`cf0ee8ba9e5dc254a06f7591d1e3482ae7c07f5d965c65811f7c2c0db33480a6`,
and was emitted in 0.006375 seconds by the release Umlaut binary with SHA-256
`85f4dedd1e3e8a8f018e3f2b4107a9879e3ecdbdc4900a8fbcf08aa6f61e7ada`.

## Adversarial cases

Every mutation received `VerifiedBad`, and the positive-only gate returned
`rejected` with exit code 1:

| Mutation | Independent reason |
|---|---|
| Principal symbol reused by the problem | `defined predicate epred1_0 is not fresh` |
| Principal symbol occurs in its own body | `definition of epred1_0 is circular` |
| Body changed but descendants retained | `ATP shows premises do NOT entail the step` |
| Definition parent replaced by an unrelated input | `ATP shows premises do NOT entail the step` |

The altered-body case is important: its changed definition remains
conservative in isolation, so the definition-shape check accepts it. Semantic
replay of the unchanged descendant is what detects the unsound proof.

## Operational path

`tools/validation/run_pinned_proofguard.py` now provides the reusable
shell-free boundary. It verifies the exact external Git identity, clean
worktree, checker and engine hashes, and a caller-declared E hash before
launching either process. Any integrity, timeout, output-shape, or process
failure emits a non-success result and cannot become `VerifiedGood`.

After adding the adapter, the Ubuntu validation-controller suite passed 42
tests with one optional Z3 probe skipped. The adapter independently returned
`VerifiedGood` for both retained positive proofs through the final uploaded
snapshot.

## Repository-wide validation

The final immutable source snapshot had SHA-256
`57f266300353652ed68d6be99246d180ba405021a84521952ebb8487e70edd44`.
On Ubuntu 24.04 with Rust 1.97.1, it passed:

- all 4,481 native Rust tests;
- formatting, strict Clippy, native release build, and native smoke runs;
- all 42 validation-controller tests, with the optional Z3 probe skipped;
- Windows GNU x64 test-target and release cross-build gates;
- the pinned first-order and higher-order E reference builds;
- 50 main compatibility cases with zero unexpected mismatches and 29
  documented expected differences;
- 216 tool compatibility cases with zero mismatches and 16 documented
  expected differences;
- all 10 benchmark behavior comparisons; and
- native Callgrind smoke runs.

The benchmark aggregate Rust/E wall-time ratio was 1.078293. The Callgrind
smoke recorded 9,609,690 Rust instructions and 7,591,871 E instructions.
Both `VALIDATION_COMPLETE` and `SUCCESS` markers were present. The ignored
full-gate archive is
`.artifacts/linode/260729-133617-70cc/full-9jt-2-10.tar.gz`, with SHA-256
`f26a3b9cf6e7cbd03192f862afd23103a87752ba9ea8dcb16207dc625b40e7a2`.

## Evidence

The passing controller report has ID
`aef91a4d3c3e8815abd3dc67d92ddd4cada650496d5d27f6a4546e16e5559d3e`.
The initial experiment snapshot SHA-256 was
`0741e7324ef9853d68d9f8c8980a88b8b5880c9aa53cc9b7ab4c02dbf11ba703`;
the post-adapter validation snapshot was
`5acd22d72c18c82f3854e791eba8c1f7a9de1e78e634c7f11c34faefd653509a`.

| Artifact | SHA-256 |
|---|---|
| Evidence archive | `153425604561aff3da1abe915731cf692f14459b42188ef06fa08658cdf5edd4` |
| `report.json` | `0d677c0284752d3ec4b8ca0405d1440083027652103516ab32844b312cbc61cf` |
| Experiment controller | `7eb2e310ed43a408c7d624d39d1e7ef778a843a74a5845ebe5c917d74f165f8e` |

The ignored local evidence archive is
`.artifacts/conservative-def-v2/results-v2.tar.gz`.

## Provisioning note

The first attempted run stopped at the upstream-test prerequisite because a
Windows-created transport archive had removed executable mode bits from the
external checker bundle. Its Python checker therefore reported that E was not
available. No experiment case ran. Restoring executable mode on the
disposable Ubuntu copy produced the passing frozen run above; no source,
binary, fixture, expected verdict, or gate changed.
