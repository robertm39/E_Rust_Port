# Independent proof, model, and soundness validation gates

## Decision

Umlaut now has a fail-closed, shell-free solution-validation controller at
[`tools/validation/validate_tptp_solution.py`](../../tools/validation/validate_tptp_solution.py).
It accepts a proof or model claim only when all local status/framing checks pass
and a separately configured semantic checker emits `SZS status VerifiedGood`.
`VerifiedBad` rejects, `Unknown` and `Timeout` become explicit coverage gaps,
and a missing checker or missing output object never becomes success.

The experiment accepts independent FOF and CNF proof checking now. It does not
pretend that complete saturation is a checkable model: Umlaut's
`Satisfiable`/`CounterSatisfiable` paths emit `Saturation`, not a finite
`Interpretation`, so those paths are recorded as gaps. ProofCheck 1.0 also
abstains on the sampled `ContradictoryAxioms` and TFF proofs, and it is not a
THF checker. Those three proof classes remain visible follow-up work.

Follow-up experiment
[`2026-07-28-001`](../2026-07-28-001-proof-checker-coverage/) now verifies
the representative `ContradictoryAxioms` proof with GAPT 2.20 and rejects its
corrupted derivative. TFF and THF remain explicit checker coverage gaps.

## Trust boundary

The gate has four layers:

1. Read the problem's `% Status :` declaration and reject a contradictory
   success claim before invoking an external process.
2. Parse non-nested, type-matched SZS output blocks and require exactly one
   nonempty proof or interpretation object for a success claim.
3. Require a false annotated formula in a refutation object.
4. Run configured checker command vectors without a shell and accept only the
   checker's positive `VerifiedGood` verdict.

This is deliberately narrower than solving the original problem again. A
second matching status does not verify the submitted derivation or model.
Non-success statuses such as `GaveUp`, `ResourceOut`, and `Unknown` are
classified `not_applicable` because they make no logical success claim.

External command vectors are JSON arrays with `{problem}`, `{solution}`, and
the extracted `{artifact}` placeholders. Checker output is bounded in the JSON
report. Exit codes distinguish verified/not-applicable (0), rejected (1),
coverage gap (2), and configuration/input error (3).

## External oracle and provenance

The Linux run used ProofCheck 1.0 from its public release archive:

- release: `AlgorithmicTruth/proofcheck-releases`, tag `v1.0`;
- archive: `proofcheck-linux-x86_64.zip`, 8,046,522 bytes;
- archive SHA-256:
  `4c4c6f71f9d8235450c6889863963ba242249c2d8d63d0461ea3acb7814b6aaa`;
- extracted checker SHA-256:
  `92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`;
- self-certification: all 117 bundled tests passed.

ProofCheck is a transient external oracle and is not committed, packaged, or
redistributed with Umlaut. Its own component licenses remain in the downloaded
archive. The release describes ProofCheck as BSD-3-Clause and invokes its
separately licensed E, Prover9/Mace4, and Vampire backends as processes. This
use does not add a product dependency or alter Umlaut's LGPL-3.0 boundary.

The protocol follows the TPTP derivation-verification model: structural
verification, faithful input leaves, a false refutation root, and semantic
checking of derived steps. The TPTP finite-interpretation guidance likewise
requires an actual interpretation before model evaluation is meaningful.

Sources:

- <https://tptp.org/UserDocs/QuickGuide/Derivations.html>
- <https://tptp.org/UserDocs/QuickGuide/FiniteInterpretations.html>
- <https://tptp.org/UserDocs/SZSOntology/>
- <https://tptp.org/CASC/J13/SystemDescriptions.html>
- <https://github.com/AlgorithmicTruth/proofcheck-releases>

## Linux results

The final run used Ubuntu 24.04, Linux 6.8.0-134, Rust/Cargo 1.97.1, and
Python 3.12.3. The uploaded worktree snapshot SHA-256 was
`1540a5c22f8f46322d94f829751a4205a7555e8141eb76e00e2e1619a448117b`;
the exact release `umlaut` SHA-256 was
`a8184939cdc05629eb252ddd89238f3703adccb6426a56e785f6fb20abf528a7`.

| Representative path | Umlaut status | Gate verdict | Independent result |
| --- | --- | --- | --- |
| FOF conjecture | `Theorem` | verified | `VerifiedGood` |
| nontrivial CNF contradiction | `Unsatisfiable` | verified | `VerifiedGood` |
| inconsistent FOF axioms with conjecture | `ContradictoryAxioms` | coverage gap | `Unknown` |
| complete FOF saturation | `CounterSatisfiable` | coverage gap | no interpretation object |
| complete CNF saturation | `Satisfiable` | coverage gap | no interpretation object |
| typed first-order conjecture | `Theorem` | coverage gap | `Unknown` |
| higher-order conjecture | `Theorem` | coverage gap | no adopted THF checker |

The nontrivial CNF fixture deliberately avoids a direct complementary input
pair. ProofCheck flags that special presentation as suspicious and abstains;
it verifies the equivalent three-clause refutation.

Adversarial outcomes:

- changing one copied FOF axiom leaf from `p(a)` to `q(a)` preserved the false
  root but produced ProofCheck `VerifiedBad`, and the gate rejected it;
- applying a valid theorem proof to a problem declared
  `CounterSatisfiable` was rejected by the status-consistency layer before
  external checking;
- Python controller tests cover missing/mismatched blocks, proof objects
  without false roots, external `VerifiedBad` for proof and model artifacts,
  inconclusive checking, missing interpretation output, and no-claim statuses.

Exact case hashes and verdicts are in [`results.json`](results.json). The 59
raw files (32,069,440 bytes) are ignored under
`.artifacts/linode/260728-043711-d2f5/`; its `summary.json` SHA-256 is
`1d345dacf47ae257876a1057b98abe879060e8862053b50be764c31e3291eae3`.
The runner and firewall were deleted after collection.

The normal comprehensive runner then validated the integrated change in run
`260728-045250-3161`, using uploaded snapshot SHA-256
`60ed205d9af97c84de730b303e8aebc5a71cbda21cefc96b1fa89e0701130495`.
All nine solution-controller tests passed alongside Rust formatting, tests,
Clippy, release builds, Windows GNU compile checks, and both C reference
builds. The compatibility suites reported zero unexpected mismatches across
50 main-program and 216 support-tool cases. All ten benchmark cases preserved
behavior, with a Rust/C wall-time ratio of `1.0780858424460409`. Its 406 raw
files (2,450,990 bytes) are ignored under
`.artifacts/linode/260728-045250-3161/`; `validation-summary.json` has SHA-256
`71a2ef9d6cbc4eb6c1f350d7a0a29e9e4dde586d7258baac3849994c0e5adbaf`.
The comprehensive runner and firewall were also deleted after collection.

## Reproduction

The tracked [`run_validation.py`](run_validation.py) harness downloads only
the pinned hash-checked oracle, self-certifies it, builds release Umlaut, runs
the seven fixtures, executes the gate, and checks both adversarial cases. Run
it only on the guarded Linux worker:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- `
        "export PATH=/root/.cargo/bin:`$PATH; cd /opt/e-rust-port/source && python3 experiments/2026-07-27-004-soundness-validation-gates/run_validation.py --repo /opt/e-rust-port/source --artifact-dir /opt/e-rust-port/artifacts/<runner-label> --source-commit <commit> --source-snapshot-sha256 <snapshot-sha256>"
}
finally {
    .\linode-runner.ps1 down
}
```

The normal comprehensive runner now also executes the nine standard-library
controller tests and preserves their log and timing.
