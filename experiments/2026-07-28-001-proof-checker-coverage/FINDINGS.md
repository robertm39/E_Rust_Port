# Independent checker coverage for ContradictoryAxioms, TFF, and THF

## Decision

Independent proof validation now accepts the representative
`ContradictoryAxioms` class. GAPT 2.20 reports `VerifiedGood` for Umlaut's
refutation, while changing the derived clause `p(a)` to `q(a)` makes GAPT
report `VerifiedBad`. The existing validation controller consequently returns
`verified` for the original proof and `rejected` for the corrupted proof.

TFF and THF remain explicit coverage gaps. Two current CASC ProoVer entrants
were tested from pinned official source releases:

- GAPT 2.20 returns `Unknown` for both typed samples.
- Nörgler 1.1 returns `Error` for both. Its
  `ConjectureNegationCheck.scala` matches only `TPTP.FOF.Logical`, contains the
  comment `todo: add other logics as well`, and raises a `MatchError` on the
  TFF and THF formulas in these runs.

Neither result is treated as verification. This is a narrower, evidence-backed
gap: Umlaut emits structurally explicit typed conjecture-negation steps, but
the adopted independent checkers do not positively check them.

## Umlaut proof-output correction

Before this experiment, question annotation and conjecture negation remained
nested under the next formula-preprocessing inference. Nörgler correctly
rejected the typed sample because the visible simplification step used the
original conjecture directly rather than presenting a distinct
`negated_conjecture` step with `status(cth)`.

Formula preprocessing now archives every changed intermediate and makes the
active formula quote it. Proof extraction therefore emits distinct records for
answer-literal annotation, conjecture negation, and later simplification. TFF
and THF regressions require exactly one explicit
`inference(assume_negation,[status(cth)],[goal])` record and require later
simplification to name that record. The answer-question regression likewise
locks the expanded ancestry.

This repair removes a real TSTP provenance defect even though the typed
semantic checkers still have implementation gaps.

## Checker bake-off

| Sample | Umlaut status | GAPT 2.20 | Nörgler 1.1 | Adopted gate result |
| --- | --- | --- | --- | --- |
| inconsistent FOF axioms plus conjecture | `ContradictoryAxioms` | `VerifiedGood` | `Unknown` | `verified` with GAPT |
| same proof with derived `p(a)` changed to `q(a)` | unchanged text framing | `VerifiedBad` at `c_0_4` | not needed | `rejected` |
| typed first-order theorem | `Theorem` | `Unknown` | `Error`/`MatchError` | coverage gap |
| higher-order theorem | `Theorem` | `Unknown` | `Error`/`MatchError` | coverage gap |

Nörgler's FOF abstention is a separate adapter defect. For the valid
FOF-to-CNF `split_conjunct` step it constructs a backend obligation whose CNF
formula has role `conjecture`; E rejects that illegal CNF role and Nörgler
reports `Unknown`. GAPT checks the same derivation independently and returns
`VerifiedGood`.

## Pinned oracle boundary

The checkers were built and run only on the disposable Linux worker. No
third-party checker source or binary is committed, packaged, linked, or
redistributed with Umlaut.

### GAPT

- release: GAPT 2.20 official CASC J13 source archive;
- license: GPL-3.0-only, retained as ignored raw evidence;
- source archive: 113,746,748 bytes;
- source SHA-256:
  `3d99d26201f6b892a167f4b8e8d8fc95b6ee76cb154155ad3854b9ea8c44b94c`;
- assembly JAR: 85,597,722 bytes;
- assembly SHA-256:
  `4532d97f9a56bd1c57bd7b127d6c1c9b8efc228faf4bd43017cfefcdea88afff`;
- build task:
  `java -Xms1g -Xmx8g -jar sbt-launch-1.11.5.jar cli/ProoVerCLI/assembly`;
- runtime:
  `timeout --kill-after=5s 180s java -Xss16m -Xms1g -Xmx8g -jar gapt.jar PROOF`.

### Nörgler and its E backend

- release: Nörgler 1.1 official CASC J13 source archive;
- license: MIT, retained as ignored raw evidence;
- source archive: 2,482,928 bytes;
- source SHA-256:
  `22cd1042af79ae1947e8478367c24a1d4b1e0208e78a49b3d8f66a222c5b9aaf`;
- assembly JAR: 6,991,650 bytes;
- assembly SHA-256:
  `29e9f5210fe9908c50cdc15f305bf08ae6930c0e768cd9eb42ae1ccd8ae1c6bf`;
- build task:
  `java -jar sbt-launch-1.11.5.jar assembly`;
- runtime: 4 GiB Java heap, 120-second checker soft limit, and 180-second
  hard wall limit with a five-second kill grace;
- backend: E 3.3.5-ho at commit
  `17026b1bfe61aaf223cfaae54947c8d2679c31a0`;
- backend binary SHA-256:
  `50a1ce2444c136f737cdc504233b32e7471de33339d9d2fc963d36ff8a02796a`.

Both builds used the 3,847,512-byte sbt 1.11.5 launcher with SHA-256
`da3424478bb0c91428bdbe621b69b4b4e86ce8d468b403656020e7ebe5f7ed84`.
GAPT selects sbt 1.12.4 from its own pinned build properties. The final runtime
was OpenJDK 21.0.11 on Ubuntu 24.04. Exact commands and path identities are in
[`results.json`](results.json).

## Evidence

The final bake-off used uploaded worktree snapshot SHA-256
`7a81b25d10dc0323ecd9422c56c06a10081383b8fe6232501998d6305104e257`.
The release `umlaut` binary was 8,177,200 bytes with SHA-256
`e6fa50170b9c964214ac9b3626192be337cdffa50d9249cfe94ae2092315f505`.

[`results.json`](results.json) has SHA-256
`9ae6ce77abc6e6a4b8ea26f41e299190f91922206f68501ab7dc46491c37ff4a`.
It indexes 31 raw files totaling 75,116 bytes. Those ignored files, including
checker logs, generated and corrupted proofs, gate reports, and retained
licenses, are under
`.artifacts/linode/260728-052723-19e1/proof-checker-coverage-v5/`.

### Comprehensive project gate

The final standard Linux runner used run `260728-065150-1ce7` and uploaded
3,549 files in a 7,352,889-byte source archive with SHA-256
`fed1fb198dab96206bde951fb1438bbb8fe79e990ac62fb2d6ec54d8c1330aa5`.
All 4,431 Rust tests, strict Clippy, formatting, the release build, nine
solution-validation controller tests, and both Windows GNU compile-only gates
passed.

The 50-case main Rust/C matrix had zero unexpected mismatches. It recorded
seven expected output-only differences: the six fixtures whose standalone
conjecture-preprocessing proof records intentionally improve on E's nested
provenance, plus the pre-existing `sledgehammer.p` difference. All 216
support-tool cases had zero unexpected mismatches, and all 10 benchmark cases
preserved behavior. The aggregate Rust/C wall-time ratio was
`1.0900992923042834`, below the `1.10` regression threshold.

The collected 442-file, 2,788,477-byte artifact set is under
`.artifacts/linode/260728-065150-1ce7/`. Its `validation-summary.json` has
SHA-256
`a35cd623ac1789f2c4ec2f2a59d4032d7a6bcd534a06be8d3e5f6f91001ecd01`.
The runner deleted both its Linode and firewall after collection.

## Reproduction

Build the pinned checker JARs and the pinned E reference only on the guarded
Linux worker, then run:

```text
python3 experiments/2026-07-28-001-proof-checker-coverage/run_bakeoff.py \
  --repo /opt/e-rust-port/source \
  --artifact-dir /opt/e-rust-port/artifacts/RUN/proof-checker-coverage \
  --source-commit COMMIT \
  --source-snapshot-sha256 SNAPSHOT_SHA256 \
  --norgler-archive /opt/checkers/Norgler-1.1.tgz \
  --norgler-jar /opt/checkers/Norgler---1.1/bin/noergler-1.1.jar \
  --gapt-archive /opt/checkers/GAPT-2.20.tgz \
  --gapt-jar /opt/checkers/gapt-runtime/gapt.jar \
  --sbt-launcher /opt/checkers/sbt-launch-1.11.5.jar \
  --eprover-ho /opt/checkers/e-cache/bin/COMMIT/ho/eprover-ho
```

The harness rejects unpinned archives, checker JARs, the sbt launcher, or the
E backend before execution. It also fails unless the positive and adversarial
FOF decisions remain `VerifiedGood` and `VerifiedBad`, respectively.

Sources:

- <https://tptp.org/CASC/J13/SystemDescriptions.html>
- <https://tptp.org/CASC/J13/Entrants.html>
- <https://tptp.org/UserDocs/TPTPLanguage/AnnotatedFormulae.html>
- <https://tptp.org/UserDocs/QuickGuide/Derivations.html>
