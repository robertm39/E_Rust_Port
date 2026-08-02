# CASC-2025 and CASC-J13 benchmark matrix and resumable batch harness

Beads: `E_Rust_Port-9jt.2.1`, `E_Rust_Port-9jt.2.7`,
`E_Rust_Port-9jt.2.11`

## Decision

Accept the manifest, strict batch protocol, ignored-corpus transfer boundary,
and report generator as the reusable competitive-evaluation substrate.
Do not infer full-matrix results from the normal-runner smoke. The high-memory
provider gate passed on 2026-08-01; the expanded canonical 8,502-run acceptance
gate remains `E_Rust_Port-9jt.2.7` until both release contracts complete.

## Manifest result

The generated
[`casc_2025_manifest.jsonl`](../../benchmarks/casc_2025_manifest.jsonl)
reconciles every row of the 12 official category result tables with exactly
one local problem:

- 2,901 problems in 12 categories and eight divisions;
- 2,425 recursively inventoried axioms;
- 4,279 include directives with no missing target;
- 100 indivisible source families;
- 1,911 train, 533 validation, and 457 test problems;
- every category represented in every split without any family crossing a
  split; and
- manifest SHA-256
  `31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.

The official CASC-30 corpus strips TPTP headers and publishes the problems in
increasing difficulty order. The manifest consequently calls its five
within-category bands an ordinal difficulty proxy and never invents numeric
TPTP ratings. SLH families come from the retained Isabelle theory path in the
`Names` header; other problems use the TPTP/entrant three-letter family.

## CASC-J13 manifest extension

The immutable
[`casc_2026_manifest.jsonl`](../../benchmarks/casc_2026_manifest.jsonl)
reconciles the official CASC-J13 ATP corpus with the published result tables:

- 1,350 problems across TNE 100, TEQ 300, FNE 100, FEQ 300, FNN 50, FNQ 100,
  and UEQ 400;
- 2,438 recursively inventoried axioms;
- 50 indivisible source families and a 935/229/186 train/validation/test split;
- exact per-problem, problem-tree, axiom-tree, and all 26 official CSV hashes;
- PRV retained as official context but excluded from the ATP problem set; and
- manifest SHA-256
  `939f8d03f0ceb0cbccd6377a01b605d84adeaa46e892a630513cccb82c825941`.

Every J13 ATP problem uses the announced 180-second wall boundary, eight
schedule cores, and 128 GiB memory contract. The ignored deterministic J13
transfer archive is 196,467,548 bytes with SHA-256
`ab89485b9d00b00e1098a3ab3184e47d10e59978320dca1f541480320e2a7fdc`.
The generalized archive tool derives its confined prefix and exact file counts
from the selected manifest instead of trusting a caller-supplied year.

The batch harness now supports session-only new-result and wall-time caps so a
large immutable contract can be safely checkpointed within the fixed-EST
high-memory allowance. These caps do not enter the contract and therefore do
not invalidate later hash-checked resume. `combined_report.py` preserves
release identity across overlapping problem identifiers and emits complete
per-release and combined coverage, overlap, status, time, and memory views.

## Runtime contract

[`batch.py`](../../tools/casc_benchmark/batch.py) runs one solver/problem pair
at a time in a fresh cgroup-v2 boundary. It measures aggregate CPU and peak
memory, enforces aggregate memory and PID ceilings, uses monotonic wall limits,
and kills the full cgroup after graceful process-session termination. It also
sets `RLIMIT_AS` in the child. SLH uses one core and an aggregate 15-second CPU
limit; wall-limited divisions use the official 120/240/480-second limits.

The immutable contract covers the manifest and selected-problem hashes,
presentation, binary hashes/revision, exact adapters, core/memory/PID limits,
Vampire seed, and optional source snapshot. Each session separately records
host and runner identity, allowing a compatible run to resume across guarded
Linodes. Existing results are skipped only after the JSON contract, problem
hash, and stdout/stderr hashes pass.

[`report.py`](../../tools/casc_benchmark/report.py) reproduces coverage by
category, division, family split, and ordinal difficulty band; classification
counts; time curves; wall/CPU/peak-memory distributions; overlap and unique
solves; status pairs; and proof/model polarity disagreements. It checks
terminal SZS statuses against independent category semantics rather than
treating either prover as an oracle. Every report warns that the checked-in
official CSVs are contextual and that this pinned local Vampire command is not
the official competition configuration.

## Ignored corpus boundary

The first smoke preflight correctly refused to run because the repository's
intentional `problems/` ignore rule kept the corpus out of source sync. That
preflight also found that the first manifest draft counted only 300 top-level
axioms and omitted 2,125 axioms in nested `ITP001` and `SET007` directories.
No solver executed in that attempt.

[`corpus_archive.py`](../../tools/casc_benchmark/corpus_archive.py) fixes the
operational gap without adding corpus bytes to Git. It creates a deterministic
regular-file-only archive, rejects absolute/traversing/link/unexpected
members, refuses overwrite, safely extracts, and then verifies all problem,
include, and recursive axiom hashes. The final ignored archive is 368,939,544
bytes with SHA-256
`efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d`.
The normal runner independently matched that hash and verified all 5,326
files after extraction.

## Normal-runner smoke

Runner `e-rust-codex-260728-112514-c164` (run
`260728-112514-c164`, Linode `101605637`) used Ubuntu 24.04.4, kernel
`6.8.0-134-generic`, four exposed AMD EPYC 9845 CPUs, and 7,940 MiB host
memory. Source snapshot SHA-256 was
`6b106c2526ce8a3fb6846df4bb0e4ac6a4514fbaef92b14a069e2dd68ba3cc2b`.
The release Umlaut binary matched
`1f94c64f49c7efeaf50c7b96db6bc61791f817e0636ebcc2fa6bd7193c0624a8`;
the uploaded Vampire matched its pinned hash.

The deliberately noncanonical contract used four cores and a 4 GiB cgroup
limit on FNE problem `KRS203+1`:

| Solver | SZS | Wall | Aggregate CPU | Peak cgroup memory | Residue |
| --- | --- | ---: | ---: | ---: | --- |
| Umlaut | `Theorem` | 0.108414 s | 0.113288 s | 30.542969 MiB | none |
| Vampire | `Theorem` | 0.261710 s | 0.245239 s | 5.484375 MiB | none |

The complete smoke report contains two of two expected results, one shared
solve, and zero polarity disagreements. Repeating the exact command produced
zero new results and hash-validated both existing results, exercising resume.
The ignored raw archive is 6,569 bytes with SHA-256
`93007b5f1b5e8de422d7516b20bc3d01112e02d4e1d040459c4341a2b551d43d`;
the tracked machine-readable digest is
[`smoke-summary.json`](smoke-summary.json).

The runner and firewall were deleted.

## J13 canonical-contract pilot

The active high-memory runner first verified the complete J13 contract
`cad062513bf08aef403550faef8e4021ea9b4528ae86a1d5b392f594f442a803`
against all 1,350 problems, then executed a session-capped 20-result pilot.
The pilot checkpointed ten results per solver and generated a valid partial
report. It also exposed a reporting defect before the long run: Vampire's
portfolio writes terminal lines such as `% (7937)SZS status Timeout`, while
the original parser accepted only an unprefixed `% SZS status ...` line.
Consequently the 179.987-second `SEV254^5` Vampire timeout was initially
recorded as an error even though its output ended with `SZS status Timeout`.

The parser now accepts the numeric portfolio prefix and classifies an explicit
`Timeout` separately from `ResourceOut` and `MemoryOut`. A regression test uses
the exact observed output shape. The pilot contract and results are retained
as diagnostic evidence only; the corrected canonical matrix starts in a fresh
output root and source-snapshot contract rather than mixing parser versions.

## J13 canonical checkpoint and host interruption

The corrected canonical run reached 185 of 2,700 J13 solver/problem results
before Ubuntu's `apt-daily-upgrade` requested a systemd manager reexecution on
2026-08-01. Systemd stopped transient service invocation
`88497424106049f9989fa26461cb298e` and immediately restarted the same service
as `7eeb7794425849a19fca91b6d28fea12`. The old cgroup became empty and no
duplicate prover workload survived, but the service restart reset the
session-wall guard and left Vampire's in-progress `SYO326^5` result without an
atomic JSON record.

The restarted service was interrupted through `SIGINT` so the batch harness's
normal `finally` cleanup emptied its cgroups. The incomplete stdout was
preserved separately, the two result-less run-root streams were removed, and
the report deterministically regenerated 185/2,700 coverage. The
hash-verified checkpoint is
`.artifacts/casc-benchmark/j13-checkpoint-260801.tar.gz` (5.9 MiB, SHA-256
`a1e84660d3b0ae1b87c9af256a58c23374f1b19b106ac11129ad80d14583ce8b`);
[`capture_260801_j13_checkpoint.sh`](capture_260801_j13_checkpoint.sh) refuses
capture while the service, a prover, or benchmark cgroup is active and retains
both systemd invocations, apt logs, the frozen binary, interrupted output, the
deterministic run archive, and verified inner hashes. The immutable matrix
remains resumable from those 185 records under `E_Rust_Port-9jt.2.7`.
Runner follow-up `E_Rust_Port-9jt.2.12` now waits for cloud-init, stops and
masks both `apt-daily` timers and services before the first `apt-get`, verifies
their inactive/masked state, and records the atomic JSON plus SHA-256 in
controller state. Comprehensive run `.artifacts/linode/260801-084455-5875/`
forced a real systemd manager reexec while a transient benchmark probe was
active: PID `3531` and invocation `e578752eab964d44a9f365c9914774ec`
survived unchanged, then SIGINT left an empty cgroup, no worker or temporary
result, and a single hash-valid coordinate that resume skipped. The lifecycle
record SHA-256 is
`0ac422ef3a8ae68b5066d4ff6d85d86ccadcb9c908635b10289fa685e9836fe8`;
the quiescence record SHA-256 is
`60e49562d214a94accb031f6b52ea25bbbc641f77a10c93b9622634fdd1c827b`.
The same clean-room run passed 4,562 library tests, every build/compatibility
gate, and all ten behavior-exact benchmarks at `1.0733692750x`, then deleted
the Linode and firewall. The checkpoint is therefore ready to resume under the
quiesced runner lifecycle.

## J13 guarded resume slice

The first quiesced-runner resume used high-memory runner
`e-rust-codex-260801-092150-91c1` (Linode `101988657`) and restored the exact
canonical J13 contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`.
Verification-only accepted all 185 retained atomic results before the new
service started. The run continued to use frozen Umlaut
`8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda`
and pinned Vampire
`3fd88f402d2b74ddf6bf96d49a2bf3c9383710b19d1c9c2c5ecb740265a5c665`;
the later parser repair is intentionally not mixed into this immutable
comparison.

Transient service `casc-j13-v2-resume-260801-092150-91c1.service` ran with
`Restart=no`, initial PID `5141`, and invocation
`1ef7992eb49c459fb73d0caf3fba5333`. A local watchdog checked that PID,
invocation, and zero-restart invariant every two minutes. The batch reached
its 17,400-second session guard normally and recorded `new=780, resumed=185`,
advancing the matrix to 965/2,700 J13 results: 483 Umlaut and 482 Vampire. The
one-result difference is the expected atomic boundary after Umlaut completed
the final coordinate and the session limit stopped before Vampire began.

The regenerated partial report is byte-reproducible at SHA-256
`17862cf3c6a51103c7259c39f14204055550d95041c4035bc64569a9507bbb89`.
It currently records 302 accepted Umlaut solves and 421 accepted Vampire
solves, with 294 shared, eight Umlaut-only, 127 Vampire-only, and zero polarity
disagreements among complete pairs. Umlaut's 76 errors are expected from the
frozen pre-repair executable and agree with the separate complete THF audit;
they are preserved rather than retroactively rerun under a different binary.

After service exit, no solver process or benchmark cgroup remained. All four
package-maintenance units were still inactive and masked, the service result
was `success` with zero restarts, and the watchdog downloaded the checkpoint
before deleting both the Linode and firewall. All eight outer hashes, all 965
result contract IDs, and every referenced stdout/stderr hash passed locally.
The ignored checkpoint
`.artifacts/casc-benchmark/j13-checkpoint-260801-092150-91c1.tar.gz` is
14,427,660 bytes with SHA-256
`1f51d7cc69744d14e36564048e02b2a77d4451e23248bb8900cf4b632020590b`.
The compact tracked record is
[`j13-resume-260801-summary.json`](j13-resume-260801-summary.json).

## J13 THF direct-lambda operand boundary

The corrected-contract slice exposed a separate release-Umlaut parser defect.
Twelve completed J13 THF cases first returned exit 3 in about 0.06 seconds with
the same `Too many arguments applied to the term` diagnostic. A complete
syntax audit then established the real boundary: the frozen release accepted
324 of 400 THF problems, falsely rejected 73 with that diagnostic, and
returned three other errors.

The common source shape applies a curried head to a direct lambda and then to
another argument. E's `applied_tform_tstp_parse` parses each `@` operand as one
literal, and `quantified_tform_tstp_parse` likewise gives an unparenthesized
lambda body one literal. Rust instead let direct lambda arguments and
quantified bodies consume a following `@` tail after their current head was
already saturated, so outer operands disappeared into the binder and the
remaining token looked like a genuine overapplication. Negated Boolean
operands had the same ambiguity in `if @ ~condition @ then @ else`. The repair
gives direct typed lambda operands a bounded right-associative path, lets
general quantified bodies consume applications only while their current head
has argument capacity, and bounds negated Boolean operands at one unit. It
also shares parenthesized logical heads such as `(&)` when they are themselves
higher-order arguments. Canonical lambda inference and expected-sort checks
remain in force; parenthesized inner applications, comma binders, explicitly
nested lambdas, conventional `![X]: p @ X` bodies, and genuine
overapplication rejection have separate regressions.

[`audit_j13_thf_syntax.py`](audit_j13_thf_syntax.py) hash-checks the immutable
manifest, all 400 THF problem files, the selected Umlaut binary, its own source,
and the source snapshot while recording every syntax-only command, return code,
wall time, and complete stdout/stderr in canonical JSON. The release-before and
corrected-after counts are recorded below only after the untouched Ubuntu corpus
runs complete. [`probe_j13_thf_proving.py`](probe_j13_thf_proving.py) then
derives its complete selection from the before-audit diagnostic class and runs
the production one-core schedule under a one-second CPU limit, failing unless
every selected input reaches a terminal proving status rather than exit 3.

The immutable Ubuntu evidence is:

- frozen release binary SHA-256
  `8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda`;
- before audit 324 accepted / 73 overapplication / 3 other errors, SHA-256
  `5dffb57022087f875795d3ed4c486762939bc440ffa6ba488273c1a5795bbc88`;
- corrected source snapshot
  `15e5ee74ea598d5ae0ff85ff4164ad1423c1d5395f24d5fd327ea1cfa690ecff`
  and release binary SHA-256
  `dfd5def3af2c7b5633f43f4e980fcd4a84e91e2de0d127d65a54321ca5dd7fc3`;
- after audit 398 accepted / 2 other errors / 0 overapplications, SHA-256
  `8457b397f123fef0bb8149acc9fbcceb1a4d8568f57bbcd6eeb64a6e1477beb7`;
  the only remaining errors are the same `SYO544^1` and `SYO545^1` Boolean
  equality diagnostics, while the former unrelated `ITP185^1` error also
  becomes accepted; and
- proving probe 73/73 `entered_proving`, SHA-256
  `14a4c40c8488b48d5484be15913b53c383ba2d437b48e101bdfd7efb0d122693`.

The four files and corrected binary are retained under
`.artifacts/casc-benchmark/`. All 142 term-bank tests, the focused executable
regressions, and release formatting passed on the corrected snapshot before
these corpus gates. Clean-room comprehensive run
`.artifacts/linode/260801-075604-b2cf/` then passed 4,562 library tests, strict
Clippy, native and Windows GNU x64 builds, clean FOL/HO C builds, 50 main and
216 tool comparisons with zero unexpected mismatches, and ten behavior-exact
benchmarks at a `1.0821091514x` aggregate Rust/C wall-time ratio.

## Guarded checkpoint controller

[`resume_j13_checkpoint.ps1`](resume_j13_checkpoint.ps1) turns each remaining
J13 slice into one fail-closed operation. Its default mode performs no network
or provider mutation: it hash-checks the explicit parent checkpoint, corpus,
frozen Umlaut, and pinned Vampire, then emits the proposed immutable contract
and resource limits. `-Execute` additionally requires a clean `main`, checks
the guarded high-memory allowance immediately before acquisition, and pins the
fresh runner, Linode, transient-service PID, invocation, and zero-restart
identity.

Before launching a prover, the controller safely restores both archives,
checks the checkpoint's inner hashes and contract file, regenerates the partial
report to validate every retained result and stdout/stderr hash, checks the
expected retained-result count, and runs the batch harness's canonical host,
corpus, binary, cgroup, and contract preflight. It monitors at most once per
minute. After service exit it rejects process, cgroup, and unit residue,
regenerates the report, embeds parentage and lifecycle evidence in a normalized
checkpoint, downloads and hash-checks that checkpoint, and only then deletes
the managed Linode and firewall. A launch-uncertain or capture-failed runner is
retained for recovery; a failure before any launch attempt is deleted.

The validated next invocation is:

```powershell
.\experiments\2026-07-28-004-casc-benchmark-matrix\resume_j13_checkpoint.ps1 `
    -CheckpointArchive .artifacts\casc-benchmark\j13-checkpoint-260801-092150-91c1.tar.gz `
    -CheckpointSha256 1f51d7cc69744d14e36564048e02b2a77d4451e23248bb8900cf4b632020590b `
    -ExpectedInitialResults 965 `
    -MaxSessionWallSeconds 14400 `
    -Execute
```

The 14,400-second batch guard reserves the projected 20-minute bank after the
2026-08-02 05:00 UTC accounting boundary for provisioning, transfer, report,
capture, and teardown. The independent service ceiling is 14,700 seconds.

## Remaining acceptance boundary

This smoke validates program construction, separate ignored inputs, binary and
corpus hashes, cgroup accounting, SZS extraction, atomic results, resume, full
report generation, artifact transfer, and cleanup. It does not validate the
required eight-core/128 GiB environment or execute all 2,901 problems for both
solvers. The earlier `g7-highmem-8` provider restriction was resolved on
2026-08-01 when the guarded lifecycle gate passed. Gate
`E_Rust_Port-9jt.2.7` now preserves the full-run acceptance work itself,
expanded to the 2,901 CASC-2025 and 1,350 CASC-J13/2026 ATP problems. The J13
manifest, archive, and combined-report contract are now ready; the 8,502
solver/problem executions and final reports remain outstanding.
