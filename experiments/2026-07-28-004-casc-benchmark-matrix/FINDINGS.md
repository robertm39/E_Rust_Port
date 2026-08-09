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

## J13 checkpoint throughput forecast

[`forecast_casc_checkpoint.py`](forecast_casc_checkpoint.py) first applies the
same outer-hash, nested-contract, result-artifact, report, inventory, and
lifecycle validation as the deletion gate, then forecasts only from those
accepted records. The reproducible 965-result invocation uses the checkpoint,
manifest, contract, `--session-seconds 14400`, and `--recent-window 100` values
recorded above. Its ignored canonical output is
`.artifacts/casc-benchmark/j13-965-forecast.json`, SHA-256
`a8e5912d772e3e2e6bb59ba280f51f534869f097b44d6ff1838ef60d314fe983`.

The first missing coordinate is Vampire on manifest record 483,
`LCL646+1.010`. All 1,735 remaining J13 coordinates have 180-second wall
limits: 35 FNE, 600 FEQ, 100 FNN, 200 FNQ, and 800 UEQ, for a deliberately
loose all-timeout upper bound of 312,300 seconds. The latest 100 completions
used 5,295.123 seconds (52.951 seconds/result), with 82 solved, 17 timeouts, and
one give-up. Their mean CPU use was 5.954 cores for Umlaut and 7.839 for
Vampire, so running coordinates concurrently would oversubscribe the immutable
eight-core contract rather than safely accelerate it.

The 100-result stationary projection is 271 new results per four-hour slice and
seven remaining slices. This is a planning estimate, not an acceptance claim:
50-, 200-, and 500-result windows project respectively 222/eight, 326/six, and
511/four. The observed slowdown therefore supports the existing fail-closed
daily checkpointing but does not justify changing the frozen execution order or
resource contract.

## Guarded checkpoint controller

[`resume_j13_checkpoint.ps1`](resume_j13_checkpoint.ps1) turns each remaining
J13 slice and the subsequent CASC-2025 slices into one fail-closed operation.
The transitional filename is retained while the armed J13 process is alive;
`-Release j13` is the default, while `-Release casc2025` selects the already
verified 2025 contract, corpus, manifest, 5,802-result boundary, service name,
and checkpoint prefix. The default mode performs no network or provider
mutation: it hash-checks the explicit parent checkpoint, release corpus, frozen
Umlaut, and pinned Vampire, then emits the proposed immutable contract and
resource limits. `-Execute` additionally requires a clean `main`, checks the
guarded high-memory allowance immediately before acquisition, and pins the
fresh runner, Linode, transient-service PID, invocation, and zero-restart
identity. Future invocations query the runner's machine-readable allowance with
the complete 14,700-second service ceiling, require both sufficient live
remaining capacity and zero active managed high-memory hosts, then retain the
broader catalog/permission check as a separate preflight.

Before launching a prover, the controller safely restores both archives,
checks the checkpoint's inner hashes and contract file, regenerates the partial
report to validate every retained result and stdout/stderr hash, checks the
expected retained-result count, and runs the batch harness's canonical host,
corpus, binary, cgroup, and contract preflight. It monitors at most once per
minute. After service exit it rejects process, cgroup, and unit residue,
regenerates both release reports plus the 8,502-coordinate combined partial
report, verifies its 4,251-problem and 66-official-CSV context, and embeds that
combined summary with parentage and lifecycle evidence in a normalized
checkpoint. It downloads and hash-checks that checkpoint and only then deletes
the managed Linode and firewall. A launch-uncertain or capture-failed runner is
retained for recovery; a failure before any launch attempt is deleted. The
future controller also writes a sibling `.validation.json` containing the exact
machine-readable local deletion-gate evidence and logs that sidecar's SHA-256.
The already armed 2026-08-02 J13 controller loaded the preceding script version;
its successor checkpoint therefore still requires an explicit local combined
report check before the next resume, whose capture will enforce this stronger
gate.

[`validate_casc_checkpoint.py`](validate_casc_checkpoint.py) supplies the final
local deletion gate for subsequent slices. It first verifies the expected outer
archive hash, rejects unsafe, duplicate, linked, or unchecksummed tar members,
checks every outer `SHA256SUMS` entry, and streams the nested run archive through
a temporary file without extracting it. It then recomputes the target contract
ID, matches the immutable manifest selection, validates every result identity
and path, hashes every referenced stdout/stderr artifact, rejects orphan
artifacts, and reconciles solver/result counts with the regenerated report and
session runner identities. Future controller invocations additionally name both
immutable releases: the local gate then requires `combined-summary.json`, fully
validates both run roots, exactly reconstructs both per-release reports and the
combined report from validated records, and requires the embedded JSON values to
match before resource deletion. The opt-in `--combined-output` bridge preserves
the single-release invocation used by the already armed controller while
allowing its legacy-shaped successor to reconstruct a missing zero-result release
summary and combined report without archive extraction. On the existing
965-result checkpoint that bridge independently reproduced the prior combined
report byte-for-byte at SHA-256
`d325fa2d64945952d7a5f713e54d2e7ff0a9a858743688ff0a7bf285810955ad`.
The validator also independently requires the Bead's analysis surface rather than
relying only on reproduction through the same report functions. Every per-release
and combined solver view must expose classification and final-status counts, time
curves, and nonempty category/division/split/difficulty/overall groups; every
group must carry coverage plus CPU, wall-time, and peak-memory distributions.
Every overlap view must carry both/unique/neither/incomplete solve counts, status
pairs, and polarity disagreements. Combined reports additionally require release
groups, both nested release summaries, and explicit official-CSV context. Removing
a combined release group or a per-release peak-memory distribution fails focused
validation. Eleven validator tests and all 52 related tests pass; the real
965-result legacy bridge passes this acceptance-surface gate unchanged.
The legacy-compatible path also retains the same contract, inner-archive,
per-release report, and 483/482 solver hashes recorded above; wrong outer hashes
and wrong expected-result counts are rejected. The validator now also parses the
outer `wc` record and full absolute result-file inventory and requires their
count and path set to equal the selected nested run; the 965-result checkpoint's
inventory hash is
`b4d5e32777b02b7ca512a72c64de05736ec979ff5f783c1e754bd5dc33a5ef1d`.
For post-capture inspection, `--expected-results` may now be omitted: the same
hash-verified outer count is used to validate the inner run and is reported with
`count_source: outer-inventory`. Controller calls retain their explicit count,
so both the caller and the archive must agree before automatic teardown.
The outer lifecycle gate also requires `Restart=no`, zero main PID and restarts,
a terminal service state, and no live batch, Umlaut, or Vampire command in the
captured process table. The real checkpoint's process and service-property
hashes are respectively `1031c60ce31cd0ae09e94c1d907d51dfc61d0e34f5650e02ca1b5c9070cb9970`
and `b298cfd3b13f34c2fb1cb576d7339701e5a62aaea42adf165ddc08ff06df2b7f`.
Ten focused unit tests additionally reject absolute, parent-traversing,
backslash, duplicate, linked, unchecksummed, contract-tampered, orphan-artifact,
missing-summary-without-opt-in, inconsistent combined, and mismatched outer
inventory/lifecycle fixtures while accepting internally consistent single- and
two-release runs.

The validated next invocation is:

```powershell
.\experiments\2026-07-28-004-casc-benchmark-matrix\resume_j13_checkpoint.ps1 `
    -Release j13 `
    -CheckpointArchive .artifacts\casc-benchmark\j13-checkpoint-260801-092150-91c1.tar.gz `
    -CheckpointSha256 1f51d7cc69744d14e36564048e02b2a77d4451e23248bb8900cf4b632020590b `
    -ExpectedInitialResults 965 `
    -MaxSessionWallSeconds 14400 `
    -NotBeforeUtc 2026-08-02T05:00:10Z `
    -Execute
```

The 14,400-second batch guard reserves the projected 20-minute bank after the
2026-08-02 05:00 UTC accounting boundary for provisioning, transfer, report,
capture, and teardown. The controller sleeps in intervals of at most 60 seconds
until the explicit UTC boundary, then reruns mutable repository and provider
preflights. The independent service ceiling is 14,700 seconds.

[`plan_next_casc_resume.py`](plan_next_casc_resume.py) is the nonmutating handoff
for later slices. Its default `auto` mode runs strict selected-run validation for
each candidate and requires exactly one release to match the SHA-bound outer
result inventory. An incomplete J13 outer run continues directly, which preserves
compatibility with the current legacy checkpoint's intentionally absent untouched
CASC-2025 summary. At the J13 boundary and for every CASC-2025 checkpoint, a full
combined pass exposes independently validated per-release counts, rejects an
advance past incomplete J13, and selects the first incomplete campaign release.
It reports `campaign_complete` without a provider query only after validating both
exact release boundaries and reproducing the embedded 4,251-problem, 8,502-result
combined report with all 66 contextual CSVs. An explicit release remains available
for diagnosis. For a continuation it queries the complete service ceiling through
`linode-runner allowance --required-seconds`; rejects mismatched schemas,
archive/contract/release identities, active managed hosts, inconsistent duration
decisions, and boundaries too far away for the controller's 24-hour guard; then
emits the exact argument vector to arm. On the current checkpoint it independently
reproduced 965 retained results and the existing 2026-08-02 05:00:10 UTC
invocation. The ignored plan captured at provider time 02:07:25 UTC has SHA-256
`1263d1f07bc095607b5194776994b793436534aa9c726f196feb18a147319cc7`;
17 focused tests cover guarded boundaries, inspection without an allowance query,
both outer-run continuations, release
transition, campaign completion, ambiguous inventories, malformed or inconsistent
validation, illegal campaign ordering, and allowance decisions. The real legacy
combined bridge now reports per-release counts `CASC-J13=965` and `CASC-2025=0`
while reproducing combined-summary SHA-256 `d325fa2d...`. Final combined validation
selects CASC-2025 so its selected run matches the 5,802-path outer inventory; it
reconstructs constituent runs in the canonical report order and spelling,
`CASC-2025` then `CASC-J13`, rather than reusing campaign order or internal keys.

Future [`resume_j13_checkpoint.ps1`](resume_j13_checkpoint.ps1) invocations call
the planner's `--inspect-only` mode before returning a dry-run plan or making any
provider mutation. The controller requires the requested release, initial count,
release boundary, checkpoint path, and archive hash to match that state. A
scheduled execution repeats the inspection after waking and after its clean-main
check, closing the gap where a direct or stale invocation could provision before
discovering a campaign mismatch remotely. The real J13 dry run reports outer
release `j13` and count 965; a premature CASC-2025 request at zero results is
rejected locally. Armed PID 20052 predates this change but was
already created from the exact validated 965-result plan and remains unchanged.

## CASC-2025 continuation readiness

The 2026-08-02 local readiness audit rehashed the complete CASC-2025 corpus
against the immutable manifest: all 2,901 problem files and 2,425 axiom files
passed. The corpus archive is 368,939,544 bytes with SHA-256
`efcebc55298d4c6770113c095e8cefdd77b9e8cbe3afa3078201f541893d1a7d`;
the manifest SHA-256 is
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.

The current combined checkpoint already carries the corrected, untouched
`casc30-2025-089e06c8-v2` run root with zero results. Its contract file SHA-256
is `f895aa07141b091060f3ee46d28f91abd6f484f3ad690630af08a7dbe34284c5`,
and its self-hashed contract ID is
`e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa`.
It selects all 2,901 manifest records and the same frozen Umlaut, pinned
Vampire, and source snapshot used by J13. Completing it requires 5,802 result
records: 1,000 SLH problems use 15 CPU seconds, 500 problems use 120 wall
seconds, 1,300 use 240 wall seconds, and 101 use 480 wall seconds. No fresh
contract construction or mixed binary is needed after J13 completes; the
successor combined checkpoint is the canonical CASC-2025 starting point. The
release-aware controller's nonmutating plans independently hash-verified both
release configurations. Regenerating the partial 2025 report at this untouched
boundary also passed with the expected 0/5,802 count, zero results for each
solver, and contract ID `e71fc642...`.

## Partial combined-report gate

The combined report remains complete-only by default, but now accepts an
explicit `--allow-partial` monitoring mode with the same missing-result and
per-release accounting used by the individual reports. It never synthesizes a
missing solver/problem coordinate. The underlying report loader now also
rejects any result identity outside the selected contract instead of allowing
an extra result count to mask a missing coordinate. The focused benchmark-tool
suite passes 23 tests, including complete combined output, partial combined
output, default rejection of an incomplete combination, and rejection of an
out-of-selection result.

The actual two-release partial report over the current combined checkpoint is
retained at `.artifacts/casc-benchmark/combined-partial-260802.json`, SHA-256
`d325fa2d64945952d7a5f713e54d2e7ff0a9a858743688ff0a7bf285810955ad`.
It records all 4,251 targeted problems and 8,502 expected coordinates, with 965
completed and 7,537 explicitly missing. The combined per-solver totals are 483
Umlaut and 482 Vampire, matching the J13 checkpoint while CASC-2025 remains at
zero. Its official context totals exactly 66 CSVs (40 CASC-2025 plus 26 J13)
and preserves the warning that local runs do not reproduce official entries or
the StarExec environment.

## Interrupted launch recovery

The 2026-08-02 05:00 UTC guarded launch provisioned high-memory runner
`260802-050016-76d3` (Linode `102066534`), but Windows PowerShell promoted the
successful bootstrap's informational `systemctl mask` stderr into a terminating
`NativeCommandError`.  The benchmark controller exited before recording the
ready runner even though the remote bootstrap completed.  The question was
whether the exact paid allocation could be recovered without trusting partial
local state or manually editing its identity record.

The controller now treats the Python process exit code as authoritative while
preserving native stderr, and `linode-runner recover` provides a narrow,
fail-closed recovery path.  It accepts only the `active`/`bootstrapping` phase,
revalidates the exact saved Linode and firewall labels, running/enabled states,
IPv4 address, available plan metadata, and SSH reachability, then reads and
validates the complete remote package-maintenance record.  It rechecks the saved
identity under the lifecycle lock immediately before committing `phase=ready`.
The CASC resume controller can explicitly claim that exact ready run ID; normal
invocations still reject every pre-existing active runner.

Exact local validation and recovery commands were:

```powershell
python tools/linode-runner/test_linode_runner.py
.\linode-runner.ps1 exec -- "date -u; ... read-only bootstrap checks ..."
.\linode-runner.ps1 recover
.\linode-runner.ps1 status
.\experiments\2026-07-28-004-casc-benchmark-matrix\resume_j13_checkpoint.ps1 `
    -CheckpointArchive .artifacts\casc-benchmark\j13-checkpoint-260801-092150-91c1.tar.gz `
    -CheckpointSha256 1f51d7cc69744d14e36564048e02b2a77d4451e23248bb8900cf4b632020590b `
    -ExpectedInitialResults 965 -MaxSessionWallSeconds 13500 `
    -ExistingRunnerRunId 260802-050016-76d3
```

All 79 runner tests passed, including a Windows subprocess regression that emits
the original successful-stderr shape, live-identity mismatch rejection, and an
identity-swap-before-commit falsification.  The real recovery reproduced remote
quiescence-record SHA-256
`627fe62915ac1c5de2e688c22f2110207f00f9311df542a989d058dacecdb345`;
all four apt maintenance units were inactive and masked, and live Linode and
firewall states were `running` and `enabled`.  The exact existing-runner dry run
then reproduced the 965-result J13 campaign boundary.  The replacement session
is reduced to 13,500 batch seconds plus a 300-second service guard so the already
consumed bootstrap time remains inside the bank-adjusted provider allowance.
This recovery proves only the host bootstrap and controller handoff; the matrix
successor remains subject to the normal service, capture, and streaming archive
validators.

The first recovered-host handoff then exposed a separate historical-contract
compatibility boundary before any solver launch.  Every corpus, binary, archive,
inner inventory, contract-file hash, partial report, and 965-result count passed,
but the current batch harness derived contract ID `ccd4e19b...` from fields whose
frozen checkpoint ID is `9f29cac7...`.  A field-by-field comparison proved that
`contract_id` was the only difference; the content remained identical.  The
controller correctly classified this as a prelaunch failure and deleted Linode
`102066534` and firewall `100863634`.

Resumes now pass `--expected-contract-id` explicitly.  The batch harness retains
an existing historical ID only when that supplied 64-hex ID matches the stored
contract and every non-ID field equals the newly derived proposal.  It still
rejects implicit compatibility, changed content, a wrong expected ID, and using
a historical ID to seed a new run.  Twenty-six reporting/batch tests, 17 planner
tests, and 11 checkpoint-validator tests passed.  A real checkpoint probe selected
`9f29cac7...` over the newly proposed `ccd4e19b...` with `non_id_equal=true`.
The next provider slice is shortened to 12,600 batch seconds plus a 300-second
service guard, preserving transfer, capture, and teardown margin after the 1,381
seconds already consumed from the 15,649-second bank-adjusted allowance.

That fresh launch completed bootstrap and saved runner
`260802-052951-15ec` as `ready`, with quiescence-record SHA-256
`a599e6284a04ab2e51aceaf563967b926d21f9ee2a51d781fd5345bb82f69876`.
While replaying the successful native command's captured output, however, the
resume logger attempted to bind an empty output record to its mandatory message
parameter and stopped before source transfer.  The runner remained idle and no
solver or batch process existed.  Both runner-output logging loops now discard
only null/empty records while retaining every nonempty stdout/stderr record; two
controller-source regressions pin that guard and the two explicit frozen-contract
call sites.  The exact ready runner can therefore be claimed by the already
fail-closed existing-runner path without another provider allocation.

The exact-host retry uploaded source snapshot `dd013dfa...`, the immutable J13
corpus, checkpoint, frozen Umlaut, and pinned Vampire.  All four outer hashes,
the 1,350-problem/2,438-axiom extraction, every inner checkpoint hash, the
frozen contract-file hash, report reconstruction, and the 965-result inventory
passed.  The combined preflight command then exited 1 before any solver or
batch service launched.  Because the runner's remote-exec path inherited SSH
stderr, the controller retained only the generic SSH exit and could not
distinguish its final batch verification from the preceding shell assertions.
The fail-closed prelaunch cleanup deleted Linode `102069265` and firewall
`100901107`; runner state is empty.

Remote exec now captures and replays both SSH streams, including them in a
nonzero-exit diagnostic.  The controller separately validates restored
summary/result inventory with explicit Python errors and invokes frozen-contract
`--verify-only` as its own logged remote phase.  A successor failure will
therefore identify the exact assertion without weakening any input, contract,
or no-solver-before-preflight gate.  Eighty-one runner tests, 20
controller/planner tests, 26 batch/report tests, 11 checkpoint-validator tests,
and the forecast regression pass; the PowerShell parser, Python compiler, and
whitespace gate also pass.

At `2026-08-02T05:52:40Z`, trusted allowance reported 13,568 seconds remaining,
below the complete 14,700-second service ceiling, and projected 27,968 seconds
of capacity at the next fixed-EST boundary.  The validated successor plan is
retained at
`.artifacts/casc-benchmark/j13-965-next-resume-plan-260803.json`, SHA-256
`43b0d90a11adf5ec25527c5717f9338f012bea4ab87f927041d15e197a095909`.
The first detached controller, PID `32600`, hash-verified the inputs but did not
survive beyond its launching execution environment; it exited silently while
sleeping and never contacted the provider.  The durable replacement is Windows
scheduled task `Umlaut-CASC-J13-Resume-20260803T050010Z`, state `Ready`, with an
exact next-run time of `2026-08-03T05:00:10Z`.  Its action is the planner's
clean-main controller invocation with a 14,400-second batch cap and
14,700-second service ceiling, an eight-hour task ceiling, and duplicate starts
disabled.  Task history is still `never run`, the transient process is absent,
and runner state remains empty.

Before that boundary, a disposable normal Ubuntu runner reproduced the frozen
preflight without consuming high-memory allowance.  Runner
`260802-234516-ad21` (Linode `102151807`, firewall `102298773`) used source
snapshot `0af1f5d7...`; all four immutable uploads, 1,350-problem/2,438-axiom
extraction, inner checkpoint hashes, contract-file hash, report reconstruction,
and 965-result inventory passed.  The new inline Python assertion then failed
because Windows PowerShell's native argument marshalling removed its embedded
double quotes before Python received the multiline SSH command.  This was an
exact reproduction of the previously hidden transport class, not a corpus or
contract mismatch.

The PowerShell runner now base64-encodes the complete `exec` command before the
native Python boundary; the Python controller strictly validates and decodes
that internal transport.  The real runner then executed the unchanged
double-quoted Python here-doc successfully and printed the exact 965-result
acceptance message.  Separating batch verification also required an explicit
`cd /opt/e-rust-port/source` because each SSH command starts a new shell.  With
both fixes, the normal host reached and failed only the intended hardware gate:
4 CPUs and 7,940 MiB cannot satisfy the canonical 8-CPU/131,072-MiB contract.
No solver launched.  The disposable Linode and firewall were deleted, leaving
empty runner state.  Eighty-four runner tests (five platform skips), 20
controller/planner tests, 26 batch/report tests, and 11 checkpoint-validator
tests pass with both PowerShell parsers, Python compilation, and whitespace
validation.

A short verification-only canonical-host run then closed the remaining
prelaunch uncertainty.  Runner `260803-000214-c49b` (Linode `102152777`,
firewall `102320675`) used pushed root commit `20eefad6` and source snapshot
SHA-256 `2a7f539356e7974f48a6a519e522c2dede272cefa08d565954e32b2aa9604404`;
its package-maintenance record SHA-256 was
`1deba3724e73b974dfa784074e4bd29dddd17623e214d0510cf2c2654f22569a`.
The exact two-phase controller preflight passed every immutable hash, corpus
extraction, inner checkpoint hash, contract-file hash, reconstructed 965-result
report/inventory, multiline assertion, historical frozen-contract comparison,
8-core/131,072-MiB host validation, and strict cgroup-v2 check.  Its terminal
message was `OK: contract 9f29cac7..., 1350 selected problems, strict cgroup v2
available`.  `--verify-only` launched no solver or service.  The diagnostic
Linode and firewall were immediately deleted.  High-memory accounting then
showed 12,900 seconds remaining today and 27,300 seconds projected at the next
boundary, still safely above the scheduled 14,700-second ceiling; the scheduled
task remains `Ready` for `2026-08-03T05:00:10Z`.  Its final scheduler audit
also requires network availability, wakes the machine, starts when available,
ignores duplicate instances, and retains the eight-hour execution ceiling.

## Durable successor scheduling

Repeated daily checkpoint slices made the manual Task Scheduler handoff a
campaign risk: a detached PowerShell process had already disappeared silently,
while the manually registered task survived.  The tracked
`schedule_casc_resume.ps1` now makes that handoff reproducible.  Its default is
nonmutating.  It requires the planner's exact schema and `ready_to_arm` state,
an incomplete canonical release total, a hash-matching checkpoint and exact
controller inside the repository, five ordered controller flag/value pairs plus
`-Execute` for a current full-fit allowance, matching batch/service durations,
and a deterministic trigger exactly five minutes after the allowance
observation.  The retained legacy path accepts six pairs only when the sixth is
an exact `-NotBeforeUtc` within 60 seconds of the old projected boundary.

Explicit `-Register` refuses replacement and creates one current-user task with
the validated UTC trigger, exact action and working directory, wake/network and
missed-start handling, battery-safe continuity, duplicate suppression, and a
finite execution ceiling.  The trigger retries every five minutes for a bounded
24-hour window.  Its action calls this scheduler with the immutable plan rather
than repeating the controller arguments.  The audited `-Launch` mode revalidates
the plan and exact task, disables that task, confirms the disabled state, and
only then invokes the validated repository controller.  A missed first boundary
can therefore retry, while the first accepted launch prevents every later retry
from duplicating either a successful or failed controller invocation.

`-Audit` resolves the task principal to its SID and requires the trigger,
repetition interval/duration, self-disabling action, principal, logon/run level,
and every guarded setting to match before emitting machine-readable evidence.
It neither embeds credentials nor trusts the human-readable task name alone.

Reproduction commands are:

```powershell
.\experiments\2026-07-28-004-casc-benchmark-matrix\schedule_casc_resume.ps1 `
    -Plan .artifacts\casc-benchmark\j13-965-next-resume-plan-260803.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\schedule_casc_resume.ps1 `
    -Plan .artifacts\casc-benchmark\j13-965-next-resume-plan-260803.json `
    -Audit
python experiments/2026-07-28-004-casc-benchmark-matrix/test_schedule_casc_resume.py
```

Six focused tests pass.  They cover default nonmutation, checkpoint/hash/path,
argument and allowance rejection, and real synthetic future tasks.  One is
registered, audited, deliberately weakened, rejected, and removed in `finally`;
another uses the current immediate full-fit plan shape, is started early to
model the first available retry, proves that it
disables itself before the deliberately invalid synthetic checkpoint reaches
the controller, and proves that a second start is refused without changing the
last-run identity.  The immediate trigger is derived from the trusted allowance
observation rather than the local clock, so preview, registration, launch, and
later audit agree on one task identity. Exact cleanup leaves no synthetic tasks
behind.

The original one-shot task's real audit passed against
plan SHA-256 `43b0d90a...`, checkpoint SHA-256 `1f51d7cc...`, J13 count 965,
trigger `2026-08-03T05:00:10Z`, exact current-user SID, and all scheduler
settings.  The ignored machine-readable audit is retained at
`.artifacts/casc-benchmark/j13-scheduler-audit-260803.json`, SHA-256
`85e86505e1060b1383b2828f72e8decec155475d345e623fbecd3ac5e0bee473`.
The tool registers only a trigger within 24 hours; it does not wait for or
automatically schedule a monthly allowance reset, replace tasks, or broaden the
controller's provider authority.  A current immediate plan remains durable
across the local PowerShell handoff, while the controller independently
revalidates the live allowance and zero-provider state before acquisition.

That original task also supplied the real missed-trigger reproduction.  The
Windows machine was unavailable at `2026-08-03T05:00:10Z`; after it returned,
the task remained `Ready`, had no next-run time, retained the sentinel
1999-11-30 last-run time, and reported `0x41303` (never run), despite
`StartWhenAvailable`.  A fresh exact audit and allowance check passed, so a
manual `Start-ScheduledTask` launched the already validated controller.  Task
Scheduler later recorded one overlapping catch-up attempt as refused
(`0x800710E0`) under `IgnoreNew`; only the original controller and one provider
runner exist.  This one-shot task predates the self-disabling wrapper and is
left untouched while it owns the live slice.

The live controller log is
`.artifacts/casc-benchmark/j13-resume-controller-20260803T135618Z-4796.log`.
It created high-memory runner `260803-135624-bc09` (Linode `102197229`, firewall
`103413524`) from clean root commit `21176a9a` and source snapshot
`d5016718...`.  Immutable upload hashes, restored 965-result inventory, report,
1,350-problem preflight, frozen contract `9f29cac7...`, 8-core/131,072-MiB host,
and strict cgroup-v2 checks passed.  At `2026-08-03T14:11:59Z`, service
`casc-j13-v2-resume-260803-135624-bc09.service` retained MainPID `3995`,
InvocationID `a38a1c59ffc94f0784a26b23f541a1b7`, zero restarts, and active/running
state; the canonical result inventory had advanced to 970.  The controller
continues to own validation, checkpoint capture, and exact resource deletion.

## Windows remote-output UTF-8 boundary

Question: can the Windows runner client relay arbitrary valid UTF-8 diagnostics
from Linux while its redirected standard streams use a narrower legacy code
page?  A read-only live probe supplied the falsification: `systemctl status`
included systemd's U+25CF black-circle marker, `ssh_command` decoded it
correctly, and Python then raised `UnicodeEncodeError` while printing through a
CP-1252 `sys.stdout`.  The remote command had already completed and did not
change provider or service state.

`linode_runner.py` now reconfigures reconfigurable standard streams to strict
UTF-8 before argument parsing or diagnostics.  The focused regression wraps
separate byte buffers in strict CP-1252 `TextIOWrapper` instances, returns
U+25CF on remote stdout and U+2713 on remote stderr, and requires both underlying
streams to contain valid UTF-8.  This also covers errors emitted after parsing;
in-memory `StringIO` test doubles remain unchanged.  The exact real reproduction
now succeeds and preserves the marker:

```powershell
.\linode-runner.ps1 exec -- `
    "systemctl status casc-j13-v2-resume-260803-135624-bc09.service --no-pager -n 4"
python tools/linode-runner/test_linode_runner.py -v
```

All 85 runner tests pass (five POSIX-only skips).  The live output reported the
same MainPID `3995`, active/running state, and 972-result inventory; raw campaign
progress remains in the controller log above.  The fix changes only local text
encoding, not SSH decoding, remote commands, lifecycle behavior, or exit-code
handling.  Its limit is deliberate: invalid remote byte sequences are still
handled by the existing SSH subprocess decoding policy rather than guessed at
this presentation boundary.

## Bounded controller probes

Question: can a connected but nonreturning SSH session prevent the guarded
controller from observing service completion and capturing a checkpoint?  The
live J13 slice supplied a direct falsification.  Controller PID `4796` entered
its `systemctl show` probe at `2026-08-03T15:54:49Z`; local Python PIDs `14704`
and `17260` plus OpenSSH PID `984` remained alive for more than seven minutes.
Remote sshd PID `47776` had no command child and waited in `do_poll`, while
separate exact probes succeeded.  The benchmark itself retained MainPID `3995`,
InvocationID `a38a1c59ffc94f0784a26b23f541a1b7`, zero restarts, and one solver
cgroup while its canonical inventory advanced from 1,306 to 1,338 results.
The failure was therefore a controller transport hang, not a service stall.

The generic `exec` dispatch previously called `ssh_command(..., timeout=None)`.
SSH's ten-second connection timeout and 15-second keepalive/four-miss policy do
not bound a session that remains connected, so the controller's own monitoring
deadline could never be evaluated.  `exec` now accepts an explicit positive
`--timeout-seconds` before the remote `--` remainder.  The PowerShell wrapper
preserves that local option while continuing to base64-encode only the complete
remote command.  The resume controller routes all short `systemctl`, result
count, archive-hash, and final-count probes through a 90-second helper; that is
strictly inside its 300-second monitoring slack.  Long preflight, launch,
capture, transfer, and cleanup operations retain their existing lifecycle
limits rather than inheriting the probe deadline.  Any future probe timeout is
reported by the CLI, logged as `controller_failed`, and follows the existing
`runner_retained_for_recovery` branch after a launch.

Reproduction and validation commands are:

```powershell
python -m unittest tools/linode-runner/test_linode_runner.py `
    experiments/2026-07-28-004-casc-benchmark-matrix/test_resume_j13_checkpoint.py -v
.\linode-runner.ps1 exec --timeout-seconds 15 -- `
    "systemctl show casc-j13-v2-resume-260803-135624-bc09.service -p ActiveState -p MainPID -p InvocationID -p NRestarts"
```

The automated nonreturning-child regression terminates locally within a
0.1-second deadline and a five-second outer assertion.  The real bounded
`systemctl show` completed in 0.8 seconds.  A deliberately isolated live
`exec -a umlaut-probe-timeout-0dbd0ca4 sleep 30` failed after 2.246 seconds,
proving the local deadline; it also established an important limit: an
arbitrary remote process can briefly survive SSH transport termination.  The
test detected and killed only that exact synthetic PID, and a second bounded
probe proved zero residue.  Consequently this facility is deliberately for
short, read-only probes, not remote workload supervision.

The already running controller loaded the preceding script, so changing files
could not repair its in-memory call.  A fail-closed adoption mode now provides
the missing ownership transfer.  It is available only with `-Execute`, the
exact existing runner ID, expected main PID, and 32-hex invocation ID; it
forbids a not-before boundary.  Normal reuse still requires runner phase
`ready`, while adoption requires the exact post-sync `synced` phase.  Before
skipping restore and launch, adoption verifies active/running state, zero
restarts, success status, the complete expected `ExecStart` command, all four
uploaded input hashes, the frozen contract-file hash, absence of both successor
checkpoint paths, and a live result count strictly after the parent checkpoint
and at or below the release total.  Any mismatch occurs on the existing
recovery-retention path and cannot delete resources.

The first scheduled adoption attempt supplied the exact identity but failed
safely before acquisition because it expected the ordinary `ready` phase; live
state proved the controller intentionally leaves the runner `synced` after
upload.  The corrected phase distinction passed six focused adoption/probe
tests and the complete 95-test runner/controller suite (five expected POSIX
skips), both PowerShell parsers, Python compilation, and whitespace checks.
Commits `fd1d8d74` and `cc84cc75` preserve the adoption implementation and
exact synced-state correction.

Dedicated one-shot task
`Umlaut-CASC-J13-Recovery-20260803T161500Z` was audited before start with the
repository controller/checkpoint, runner `260803-135624-bc09`, MainPID `3995`,
InvocationID `a38a1c59ffc94f0784a26b23f541a1b7`, current-user limited principal,
network/wake/start-available settings, `IgnoreNew`, and an eight-hour ceiling.
It was started once and immediately disabled while running.  Recovery log
`.artifacts/casc-benchmark/j13-resume-controller-20260803T161623Z-13256.log`
then verified the exact `ExecStart`, all uploaded hashes, contract hash, and
1,351-result inventory before recording `existing_service_adopted`.  Its next
bounded poll completed normally at `2026-08-03T16:17:30Z`.

Only after that proof, the original task was stopped and disabled.  Old PID
`4796` and its stuck child tree are absent; the old task has no next run and
records `0x41306` (terminated).  Recovery PID `13256` remains responsive and
owns the unchanged remote PID/invocation with zero restarts and one active
solver cgroup.  The current slice still requires its final verified checkpoint
capture and resource deletion before this recovery is accepted.

## Unloaded transient-service completion identity

Question: can checkpoint capture still prove the exact completed service after
systemd garbage-collects a successful transient unit?  The recovered J13 slice
supplied the concrete case.  It completed naturally at 1,457 results
(`new=492, resumed=965`) with exit status zero and no restart.  The next bounded
poll returned `LoadState=not-found`, inactive/dead, MainPID zero, and an empty
`InvocationID`; the first recovery controller therefore failed closed with
`Transient service invocation changed` and retained runner
`260803-135624-bc09`.  No batch or solver process remained, and neither remote
successor-checkpoint path existed.

The journal retains the authoritative terminal chain even after the unit object
is unloaded.  Process records name unit
`casc-j13-v2-resume-260803-135624-bc09.service`, PID `3995`, invocation
`a38a1c59ffc94f0784a26b23f541a1b7`, boot
`c1a84dc2dd8c45828c83c87295d9d35f`, and the complete expected batch command.
A later manager record has systemd's successful-unit `MESSAGE_ID`, the same
unit and `INVOCATION_ID`, and the exact `Deactivated successfully` message.

Completed-service recovery is now explicit through
`-AdoptCompletedService`; it is mutually exclusive with live-service adoption
and retains the same exact runner, PID, invocation, input hashes, frozen
contract, result-range, and collision requirements.  It additionally requires
one boot identity, one invocation identity, the exact PID/command, exactly one
contract-bound batch completion summary, and exactly one later successful
terminal manager record.  The journal-reported total must equal both the live
result inventory and the final downloaded count.  Capture preserves the raw
JSON journal and terminal boot/count metadata.  Completed recovery reserves a
bounded 1,800-second allowance instead of pretending that an already finished
service needs another complete 14,700-second runtime.

The embedded verifier was run directly against the retained live journal and
returned 1,457 results with the identities above.  Independent probes proved
zero matching batch/solver processes, both remote checkpoint paths absent, and
11,691 seconds of remaining guarded high-memory allowance.  Eight focused
tests cover the normal terminal record plus mixed invocation, wrong command,
missing/duplicate success, terminal-before-summary, and mixed-boot
falsifications.

The patched completed-service controller at clean main commit `0ada6162`
repeated every identity/hash/count/collision gate, regenerated the CASC-2025,
J13, and combined reports, and captured
`.artifacts/casc-benchmark/j13-checkpoint-260803-135624-bc09.tar.gz`.  Its
17,524,998 bytes hash to
`72dcf94ffa7c0c8f8d5c7027a8118c20f29e61ca5a2e822d0cd24f7e791ab7e1`.
The streaming validator proved 1,457/2,700 J13 results (729 Umlaut, 728
Vampire), 1,457/8,502 combined results, 4,251 targeted problems, all 66
contextual CSVs, a 4,451-member inner archive, success/zero-restart lifecycle,
and complete report acceptance surfaces.  Controller and independent sidecars
are byte-identical with SHA-256
`8ebf9dc4f4d6da2d531b6719339f0370ab91f8e2e81caf56fd472bc24d9b2745`.

An additional independent archive audit verified all 12 outer hashes, 13
regular members, 1,457 sorted unique result paths, 497 JSON journal records,
terminal sequence 15,491, the exact invocation/PID/boot/command chain, masked
maintenance, and empty solver-unit/cgroup residue.  Controller log
`.artifacts/casc-benchmark/j13-resume-controller-20260803T181948Z-4924.log`
records `checkpoint_verified` followed by deletion of exact Linode
`102197229`, firewall `103413524`, and `managed_resources_deleted`; runner
status is now active null with no parked host.  Both obsolete disabled task
definitions were removed after their final results were recorded.

The successor planner independently selected J13 1,457/2,700 from that archive.
At `2026-08-03T18:22:14Z`, only 11,481 of the required 14,700 seconds remained,
so it emitted the unavailable-now plan
`.artifacts/casc-benchmark/j13-1457-next-resume-plan-260804.json`, SHA-256
`50c8bcbad1002cd953759d4bc943fd8ef0450b5140107bc4c3be51e15d0d4458`,
for `2026-08-04T05:00:10Z`.  Real task
`Umlaut-CASC-J13-Resume-20260804T050010Z` is registered and independently
audited: enabled/Ready, current-user limited principal, exact plan-bound
self-disabling action, five-minute retries for one day, wake/network/
start-available policy, `IgnoreNew`, and eight-hour ceiling.  This supplies the
real successor gate required by the missed-trigger repair while leaving the
incomplete matrix campaign active.

## Session-wall cgroup teardown race

Question: can a terminal J13 slice retain its completed results without
weakening the cgroup-residue deletion gate when cgroup v2 reports a transient
`EBUSY` during teardown?  The guarded 9,000-second slice on runner
`260803-182617-21a7` reproduced that boundary.  Service
`casc-j13-v2-resume-260803-182617-21a7.service`, MainPID `3997`, invocation
`5f0221c8996441daae9ed0a06e3fd45b`, and boot
`7ff4be2759d64125934fc2107d815b34` advanced the frozen J13 run from 1,457 to
1,531 records.  At the session wall, cleanup of
`/sys/fs/cgroup/umlaut-casc-3997-766-vampire-8b69c10d` received errno 16.
The batch exited 2; systemd journal records 13,075 and 13,076 prove the exact
`ExecStart` exit and later `exit-code` unit failure.  The controller regenerated
all partial reports but correctly rejected its nonempty cgroup inventory and
retained Linode `102210235` plus firewall `103722650`.

A later bounded probe falsified a live-process interpretation: `cgroup.procs`
and `cgroup.threads` were empty, `cgroup.events` reported `populated 0`, no
batch/Umlaut/Vampire process or solver unit remained, and the failed service
still had `NRestarts=0`, `Result=exit-code`, and `ExecMainStatus=2`.  The
partial checkpoint contains ten expected regular files, including the
18,717,247-byte inner run archive and a 1,531-line result inventory; capture
stopped before input hashes, resume metadata, outer hashes, or the outer
archive could be created.

The batch cleanup now waits for both an empty PID inventory and an explicit
`populated=0`, then retries only `EBUSY`/`ENOTEMPTY` removal failures within its
existing two-second monotonic deadline.  Persistent population, missing
population proof, persistent busy state, and unrelated removal errors remain
fatal.  The recovery controller adds a separate failed-service adoption mode.
It requires exact failed properties plus one journal boot/invocation,
PID/command, `ExecStart` exit status, later unit-result record, uploaded hashes,
contract hash, and result inventory.  Optional partial-capture cleanup is
restricted to that mode and verifies the exact directory inventory, live and
partial counts, a PID-prefixed cgroup path, empty procs/threads, and
`populated=0` before removing only that cgroup and checkpoint directory.  A
terminal recovery reserves 600 seconds; previous verified captures completed
in well under a minute, while the controller still fails closed if the trusted
allowance cannot cover that bound.

Reproduction and focused validation from the repository root:

```text
python -m unittest tools.casc_benchmark.test_casc_benchmark
python experiments/2026-07-28-004-casc-benchmark-matrix/test_resume_j13_checkpoint.py
[System.Management.Automation.Language.Parser]::ParseFile(...resume_j13_checkpoint.ps1...)
git diff --check
```

The 29 batch/report tests and initial 10 controller tests pass.  The cleanup tests
simulate a delayed empty `EBUSY`, permanently populated state, and persistent
empty-but-busy state; the journal tests falsify wrong command, exit status,
unit result, terminal ordering, and duplicate failure evidence.  Live recovery
then passed every failed-service and partial-residue gate and downloaded outer
archive `e99d03fe526742f4c9716e90dfcff8ad1cab1c28b50c41a78fc102e146645d2a`,
but independent validation found exactly two unreferenced streams:
`results/vampire/feq/0766-feq-8b69c10d0c20.stdout` and `.stderr`.  No matching
result JSON exists; these are the interrupted Vampire pair named by the failed
cgroup.  The validator failed closed and again retained the runner.

Capture now scans only the target run's result tree after the zero-process
gate.  It preserves every stream with a result JSON, removes an unreferenced
base only when both its `.stdout` and `.stderr` regular files exist, and rejects
symlinks or an ambiguous one-stream set.  This makes failed-terminal recovery
match the batch resume path, which already removes result-less streams before
rerunning a solver/problem pair.  A focused structural regression pins the
process-check/cleanup/report ordering and both-stream requirement.  A fresh
retry removed exactly two streams from one base and captured
`.artifacts/casc-benchmark/j13-checkpoint-260803-182617-21a7.tar.gz`,
18,569,376 bytes with SHA-256
`80c052b18e740c242311de7ca31ad5aa8c770460ee3164f4c6187c6b81332870`.
The streaming validator proves 1,531/2,700 J13 records (766 Umlaut, 765
Vampire), 1,531/8,502 combined records, all 66 contextual CSVs, and failed
service lifecycle `exit-code`/status 2 with zero restarts.  Controller and
independent validator sidecars are byte-identical at SHA-256
`107cc6c6f100784898319cb927371070ebd82a4bc30b14a9e0b645b76793157f`.

An independent raw audit verified the archive hash and all 12 outer hashes,
13 outer and 4,675 inner regular members, inner SHA-256
`998d972dd736ba2f372f79b3a6eb02471ebf447c4f81fa8eda56f7adcfe7978c`,
1,531 unique sorted result paths, all 3,062 referenced streams with zero
orphans, 80 single-boot/single-invocation journal records, exact process-exit
sequence 13,075 followed by unit-failure sequence 13,076, masked package
maintenance, and empty cgroup/solver-unit inventories.  The rejected archive
was preserved with suffix `.invalid-orphan-artifacts.tar.gz` for diagnosis.
Only after `checkpoint_verified`, the controller deleted exact Linode
`102210235` and firewall `103722650`; managed-runner status is active null with
no parked hosts.

The successor planner selected J13 1,531/2,700.  At
`2026-08-03T21:23:40Z`, 955 of the required 14,700 seconds remained, so plan
`.artifacts/casc-benchmark/j13-1531-next-resume-plan-260804.json`, SHA-256
`f9eac6289137b88f907d4ea7df9efe43a8454a85247c46225733b8bde96ad7aa`,
targets the guarded `2026-08-04T05:00:10Z` boundary.  The stale 1,457-result
task was audited and replaced by the same one-shot name,
`Umlaut-CASC-J13-Resume-20260804T050010Z`; a post-registration audit proves the
new archive/hash/count, self-disable-before-controller action, five-minute
one-day retries, limited current-user principal, wake/network/start-available
settings, `IgnoreNew`, and eight-hour ceiling.

## Pre-controller scheduled-launch evidence

The 1,531-result task fired at exactly `2026-08-04T05:00:10Z`, disabled itself,
and returned task result 1 before provisioning.  At the first post-trigger
audit there was no matching controller process or controller log, no active or
parked runner, and no new artifact.  A direct nonexecuting controller plan
subsequently revalidated the archive, count, campaign state, and every immutable
input.  Beads also exported the completed `E_Rust_Port-9jt.2.7.6` row over its
stale committed `in_progress` JSONL row, but that was not yet sufficient to
attribute the launch failure; no provider contact occurred.

The scheduled launcher now creates a unique ignored launch log before checking
or disabling the task.  It records the task and plan hash, the successful
self-disable, controller invocation, nonempty controller output as it arrives,
and a terminal completion or exception.  The controller's own log remains the
authoritative provider/run record after its clean-main gate.  Functional tests
execute synthetic successful and failing controllers and prove both terminal
records; the real Task Scheduler test proves a deliberately invalid checkpoint
still self-disables before controller execution while preserving the complete
failure chain.  Test-created tasks and logs are removed afterward.

The first logged retry then supplied the missing exact failure: array splatting
treated the validated `-Flag, value` sequence as positional arguments, binding
`j13` to `CheckpointSha256`.  This is also the cause of the original silent
task result 1; it fails during PowerShell parameter binding, before the
controller body and its clean-worktree check.  The scheduler now reconstructs
a named-parameter hashtable from the already-validated six flag/value pairs and
sets `Execute` as a switch before invocation.  Synthetic hashtable-bound success
and failure tests plus the real invalid-checkpoint task path cover the corrected
binding without permitting unvalidated parameters.

The later calendar-month allowance policy made every fresh full-fit planner
result immediate and left the durable launcher accepting only the obsolete
future-boundary shape.  Immediate plans now retain the same audited Task
Scheduler handoff: their five exact flag/value pairs are named-splatted, their
task identity uses a deterministic trigger five minutes after the trusted
allowance observation, and the launch still self-disables before invoking the
controller.  No-fit monthly results remain informational and cannot be armed.
The six scheduler tests include a real immediate-plan registration and forced
launch through the deliberately invalid checkpoint path; the complete terminal
failure log and refusal of a duplicate start are preserved.

The first production immediate handoff used plan
`.artifacts/casc-benchmark/j13-1531-next-resume-plan-260805-2240.json`, SHA-256
`84cd144b35413f7b0a5308ff3dd2a941c41b9174a7c0ce898040a67737ee3df5`.
It independently selected J13 1,531/2,700, required 18,000 whole-hour-billed
seconds for the 14,700-second service ceiling, and observed 288,000 seconds
remaining.  Task `Umlaut-CASC-J13-Resume-20260805T224538Z` passed its exact
post-registration audit, then launch log
`.artifacts/casc-benchmark/scheduled-launch-j13-20260805T224034Z-20004.log`
proved self-disable and named controller invocation.  Exactly one runner was
created: `260805-224055-fa64`, Linode `102345835`, firewall `107959971`.

The runner quiesced package maintenance at SHA-256
`a945562780e72aaeaecb02a4fd41a6d9863967b606d7b14953f24284f5bc84a7`
and uploaded a 4,184-file source snapshot rooted at commit `9e28e2af`, archive
SHA-256 `91bb088f9e8b5374a1de6d2b149213f7066b2afdd917187ec67ce46cec744198`.
All frozen inputs, 1,350 problems, 2,438 axioms, the restored 1,531-result
inventory, and contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`
passed before service
`casc-j13-v2-resume-260805-224055-fa64.service` started with MainPID `3971`,
invocation `e9bbde41c8894c82ae538eb646535ed9`, and zero restarts.  This production
retry crosses the original pre-controller failure boundary without a duplicate
provider resource; checkpoint capture remains part of the parent campaign.

## Headless scheduled launches and orphan-plan recovery

Question: why did a PowerShell console repeatedly take focus every five
minutes, and can it be stopped without weakening the missed-trigger retry or
interrupting the live J13 controller?  The exact Windows task inventory found
one enabled synthetic task,
`Umlaut-CASC-J13-Resume-20260805T041500Z`, whose action referenced deleted test
plan `.artifacts/casc-benchmark/tmp0xlibvr3/plan.json`.  It last failed at
`2026-08-05T22:50:01Z` with result `0x00000001` and had another `PT5M` retry
scheduled for `22:55:00Z`.  Its limited `Interactive` principal ran ordinary
`powershell.exe`, placing every retry in the logged-in desktop session.  No
`Umlaut-Linode-Reaper-*` task or parked runner existed, falsifying the reaper
itself as the immediate source.

The synthetic registration test used a plan 23 hours in the future and removed
its temporary directory when the creating process ended.  Its `finally`
cleanup suppressed unregister errors and verified only the cleanup
PowerShell's exit code, so an interrupted or failed cleanup could leave a task
that appeared successfully cleaned.  The scheduled script then validated the
external plan before reaching `Disable-ScheduledTask`; a missing plan therefore
failed on every retry without disabling the task.  After revalidating its exact
name, action, repository, deleted temporary plan, description, principal, and
retry shape, the orphan alone was unregistered.  The production task
`Umlaut-CASC-J13-Resume-20260805T224538Z` remained disabled-but-running with its
controller process and provider ownership unchanged.

New CASC actions bind the canonical task name explicitly and run
`powershell.exe -WindowStyle Hidden`.  Launch first validates a plan-independent
envelope containing the exact name/timestamp, action script, lexical plan path,
working directory, description, current-user SID, trigger, retry policy, and
guarded settings.  It disables and revalidates that exact task before reading
the plan.  The existing immutable plan/hash/controller validation then runs and
must reproduce the same name, path, and disabled task before provider work.
Missing or invalid JSON plans now leave a durable launch log and fail once with
no controller invocation.  Action drift still fails before disablement.  The
Linode reaper action uses the same hidden window while retaining its limited
interactive principal: S4U would remove network and encrypted-file access,
which the API/SSH workflow and user-scoped DPAPI token require.

The Windows test harness now tracks every synthetic identity, stops and
unregisters it before deleting temporary input, verifies actual absence, and
scavenges an older task only when all canonical production fields match and
its plan is a missing `tmp*` artifact.  A triggerless, uniquely named live task
proved a hidden PowerShell process existed with zero visible matching windows.
Separate Task Scheduler regressions prove valid-plan launch, missing-plan and
corrupt-plan single failure, action drift refusal, duplicate-start refusal,
and zero retained synthetic tasks.  Reproduction from the repository root:

```powershell
$tokens=$null; $errors=$null
[Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path experiments/2026-07-28-004-casc-benchmark-matrix/schedule_casc_resume.ps1),
    [ref]$tokens,
    [ref]$errors
) | Out-Null
.\.venv\Scripts\python.exe `
    experiments/2026-07-28-004-casc-benchmark-matrix/test_schedule_casc_resume.py -v
.\.venv\Scripts\python.exe tools/linode-runner/test_linode_runner.py -v
```

All 11 scheduler tests pass in 114.529 seconds.  All 91 runner tests pass with
five expected POSIX-only skips; their relocated-wrapper cases now copy the
active test interpreter instead of relying on a machine-wide Windows App
Execution Alias.  The surrounding benchmark, package, checkpoint validation,
resume-controller, resume-planning, and forecast suites add 87 passing tests.
Both PowerShell scripts parse, Python compilation and `git diff --check` pass,
and post-test inventory contains no synthetic CASC or Linode reaper task.
Historical disabled production tasks remain as audit evidence; this change
neither removes them nor alters an already-running controller.  A final bounded
live probe at `2026-08-05T23:22:23Z` found the preserved service active and
running with MainPID `3971`, invocation
`e9bbde41c8894c82ae538eb646535ed9`, zero restarts, and 1,546 results, 15 more
than its restored checkpoint.  Linode `102345835` remained running behind
enabled firewall `107959971`, with no parked resources.

## Verified J13 checkpoint at 1,663 results

The guarded successor finished without a restart at
`2026-08-06T02:53:23Z`.  The unloaded-unit journal proof binds MainPID `3971`,
invocation `e9bbde41c8894c82ae538eb646535ed9`, boot
`628b5ced1b50438180f2ce79d982084c`, and success sequence `13240` to the exact
service.  Its terminal summary reconciles 1,531 resumed plus 132 new results
to 1,663 total.  Capture found no result-less stdout/stderr pair, regenerated
the partial J13 and combined reports, and found no solver unit or cgroup
residue.

The downloaded ignored archive is
`.artifacts/casc-benchmark/j13-checkpoint-260805-224055-fa64.tar.gz`: 22,680,371
bytes, SHA-256
`1d6d2934ab0e15c635148eab6c7ae7478f6f321b7b2bfce8488e4580d41ad3c8`.
Its 13-member outer envelope binds a 1,663-path inventory SHA-256
`ffacfcd883f8b3ad4c7f0ef0d0433c4368bf17ecae06997b9f5b65def5bdc2ca`.
The inner archive has 5,073 regular members and SHA-256
`bf8c646240150681702d2b1aa8c18687b23f6b4a6fcc24d661fe46d11320023f`.
Strict validation reproduces contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`,
832 Umlaut plus 831 Vampire records, 1,663/2,700 J13 results, 0/5,802
CASC-2025 results, and 1,663/8,502 combined results.  The J13 and combined
summary hashes are respectively
`76480c60f72cf9b9a6d07e57dfd1dc130c99c07b7c9e68001bfe0f386f15d976`
and `986dfa42c19f8586b7ebef816849c35f771296e6f76cfae090adab25258510b6`.

The controller sidecar and a separately invoked full validator are
byte-identical, both SHA-256
`1d0f108b0b9763258af8e553a300df7b710ff958326c843b0fd20af8de68cf2d`.
Reproduction from the repository root:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/j13-checkpoint-260805-224055-fa64.tar.gz `
  --archive-sha256 1d6d2934ab0e15c635148eab6c7ae7478f6f321b7b2bfce8488e4580d41ad3c8 `
  --manifest benchmarks/casc_2026_manifest.jsonl `
  --run-name casc-j13-2026-089e06c8-v2 `
  --contract-id 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --expected-results 1663 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/j13-checkpoint-260805-224055-fa64.tar.gz.independent-validation.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260805-224055-fa64.tar.gz `
  --checkpoint-sha256 1d6d2934ab0e15c635148eab6c7ae7478f6f321b7b2bfce8488e4580d41ad3c8 `
  --inspect-only
```

The inspect-only campaign gate selects outer/next release `j13` at exactly
1,663 results, leaving 1,037 J13 records before any CASC-2025 transition.
After `checkpoint_verified`, the controller deleted Linode `102345835` and
firewall `107959971`; read-only runner state is `active: null, parked: []`.
The launch task is disabled with result zero, its controller process is absent,
and its terminal log ends with `controller_invocation_completed` and
`task_launch_completed`.

## Active successor from the 1,663-result checkpoint

After checkpoint evidence was pushed at root commit `1aac2811`, the auto
planner revalidated 1,663/2,700 J13 results and observed 270,000 monthly
allowance seconds at `2026-08-06T03:00:58Z`, enough for the complete 14,700
second service ceiling.  Ignored plan
`.artifacts/casc-benchmark/j13-1663-next-resume-plan-260806.json` has SHA-256
`5aa305d9cef92828b0714ac43b75bccac917cf6ce8e90de53681d34b6ce94aea`.
Its hidden, limited-interactive task
`Umlaut-CASC-J13-Resume-20260806T030558Z` passed exact audit, triggered once at
`03:05:58Z`, disabled itself before the controller, and revalidated the plan
before provider contact.

The controller rechecked allowance and provisioned exactly one successor:
runner `260806-030618-c092`, Linode `102354657`, firewall `108358859`.
Package-maintenance quiescence SHA-256 is
`b3001f3770b878f1f2a067316fa3e2897318862a16c3932ddd8f4e58160f8877`;
the 4,184-file source snapshot binds root commit `1aac2811` and archive SHA-256
`6e17f0ff8e7a5fcb3019181a73078a90908fb0f7307c16c34273f110d9a84f4e`.
All four frozen inputs uploaded, the 1,663-result restore and immutable-contract
preflight passed, and service
`casc-j13-v2-resume-260806-030618-c092.service` started with MainPID `3994`,
invocation `c20d2f53498c421bbbe379b43519f547`, and zero restarts.  The task is
disabled-but-running and retains controller ownership until the next validated
checkpoint.

## Transport-only controller recovery at 1,776 results

Question: can a bounded SSH probe failure transfer controller ownership without
restarting the solver, accepting weaker identities, or deleting the retained
provider resources?  The active successor supplied a second real transport
failure.  Original controller log
`.artifacts/casc-benchmark/j13-resume-controller-20260806T030612Z-8104.log`
last observed 1,764 results with MainPID `3994`, invocation
`c20d2f53498c421bbbe379b43519f547`, and zero restarts.  Its next exact
`systemctl show` probe timed out locally after 90 seconds at
`2026-08-06T05:39:05Z`.  The controller logged `controller_failed` and
`runner_retained_for_recovery`; it did not capture a checkpoint or delete
Linode `102354657` or firewall `108358859`.

A separate bounded read-only probe returned immediately and falsified a
service failure: the exact unit remained loaded, active/running, MainPID
`3994`, invocation `c20d2f53498c421bbbe379b43519f547`, `NRestarts=0`, and
success status.  Its result inventory had advanced to 1,776.  The strict local
planner independently revalidated parent checkpoint
`.artifacts/casc-benchmark/j13-checkpoint-260805-224055-fa64.tar.gz`, SHA-256
`1d6d2934ab0e15c635148eab6c7ae7478f6f321b7b2bfce8488e4580d41ad3c8`,
as the exact 1,663-result J13 resume candidate.

Recovery used hidden, limited-interactive task
`Umlaut-CASC-J13-Recovery-20260806T054137Z`.  Its ignored wrapper
`.artifacts/casc-benchmark/recover-j13-20260806T054137Z.ps1` parses without a
PowerShell error and hashes to
`ecdaada7edccc7ca7e095a3530a226d55fbb6446e76dabb035b239bc515ad2a9`.
Before controller invocation it disabled its own exact task and required clean
synchronized `main`.  Registration audit proved a hidden PowerShell action,
the exact wrapper and working directory, current-user `Interactive`/`Limited`
principal, network/wake/start-available settings, `IgnoreNew`, five-minute
one-day retries, and an eight-hour ceiling.  The forced first start showed the
task disabled while its recovery process remained running.

Recovery controller log
`.artifacts/casc-benchmark/j13-resume-controller-20260806T054259Z-16036.log`
verified 259,200 remaining allowance seconds against the complete 14,700-second
ceiling, then required the exact runner, unit, MainPID, invocation, zero
restarts, complete `ExecStart`, all four uploaded hashes, frozen contract-file
hash, absent successor paths, and a result count strictly after the parent.
Only after every gate passed did it record `existing_service_adopted` at 1,776
results.  Launch log
`.artifacts/casc-benchmark/scheduled-recovery-j13-20260806T054137Z-16036.log`
preserves the wrapper handoff.  The next ordinary poll advanced to 1,778 with
the same service identity.

Exact read-only diagnosis and local validation commands were:

```powershell
.\linode-runner.ps1 status
.\linode-runner.ps1 exec --timeout-seconds 90 -- `
  "systemctl show casc-j13-v2-resume-260806-030618-c092.service --property=LoadState,ActiveState,SubState,MainPID,InvocationID,NRestarts,Result,ExecMainStatus"
.\linode-runner.ps1 exec --timeout-seconds 90 -- `
  "find /opt/e-rust-port/casc-runs/casc-j13-2026-089e06c8-v2/results -type f -name '*.json' | wc -l"
python experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --inspect-only `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260805-224055-fa64.tar.gz `
  --checkpoint-sha256 1d6d2934ab0e15c635148eab6c7ae7478f6f321b7b2bfce8488e4580d41ad3c8
```

This proves an exact, no-restart ownership transfer after a transport-only
failure.  It does not accept the active slice: checkpoint capture, independent
validation, zero-residue proof, and provider deletion remain outstanding.

## Verified recovered J13 checkpoint at 1,855 results

Question: did the transport-recovered controller preserve the original solver
process through a valid successor checkpoint, and is that checkpoint safe to
resume after independent validation and provider teardown?  The adopted service
finished successfully at `2026-08-06T07:18:57Z`.  Its terminal journal binds
MainPID `3994`, invocation `c20d2f53498c421bbbe379b43519f547`, boot
`813e80b2f2f04b2185e7dc93de3f159d`, and success sequence `20006` to the exact
unit.  The batch's terminal message reconciles 1,663 resumed plus 192 new
results to 1,855.  It had zero restarts and unloaded as inactive/dead with
`Result=success` and `ExecMainStatus=0`.

Capture removed no incomplete result artifact, regenerated both per-release
summaries and the combined report, and found no surviving solver unit or cgroup.
The ignored archive is
`.artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz`: 28,228,858
bytes, SHA-256
`c9f54dadfab3e28f95e1d6d0fd8b16d474e0dbc5d6ac6d27dc6dc00cc313d012`.
Its 13 regular outer members bind a unique, sorted 1,855-path result inventory,
SHA-256
`b83242ab8bd09866f1206d9a3d23aecd3bd34246d905c6fd27053cf8a3261c0a`.
The nested archive is 44,528,377 bytes with 5,651 regular members and SHA-256
`94634baf50b23a80f39561a18997bc56b3ea4731539b65a9775aabb8c6213063`.

Strict validation reproduces contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`,
928 Umlaut plus 927 Vampire records, 1,855/2,700 J13 results, 0/5,802
CASC-2025 results, and 1,855/8,502 combined results.  The J13 and combined
summary hashes are respectively
`2d6af8c69b475a245ffb4bfe69cb38787bc1e48f4d28e7f0f644c220172dfb2e`
and `2f6c1de861d9b51a08271fb82c7f7399ac7f57d4cfbbf9d2b40cdd5700ab5e68`.
Both solvers retain classification/status counts, time curves, and
category/division/split/difficulty/overall coverage with CPU, wall-time, and
peak-memory distributions.  Overlap retains both/unique/neither/incomplete
counts, status pairs, and polarity checks.  The combined report contains both
releases, all 4,251 targeted problems, 40 plus 26 contextual official CSVs, and
the explicit warning that local runs do not reproduce official StarExec
entries.

The controller sidecar and a separately invoked validator are byte-identical:
2,326 bytes and SHA-256
`3f0d94890bcc0abf771ebf35b86b4ec8a60fe1fe8536701d5530db252a9107d6`.
Reproduction from the repository root:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz `
  --archive-sha256 c9f54dadfab3e28f95e1d6d0fd8b16d474e0dbc5d6ac6d27dc6dc00cc313d012 `
  --manifest benchmarks/casc_2026_manifest.jsonl `
  --run-name casc-j13-2026-089e06c8-v2 `
  --contract-id 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --expected-results 1855 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz.independent-validation.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz `
  --checkpoint-sha256 c9f54dadfab3e28f95e1d6d0fd8b16d474e0dbc5d6ac6d27dc6dc00cc313d012 `
  --inspect-only
tar.exe -tzf `
  .artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz
tar.exe -xOzf `
  .artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz `
  j13-checkpoint-260806-030618-c092/SHA256SUMS
.\linode-runner.ps1 status
```

As a falsification check independent of the validator summary, the nested
archive was extracted under
`C:\tmp\umlaut-casc-audit-260806-030618-c092`.  PowerShell enumerated 1,855
unique solver/problem coordinates, matched their exact paths to the outer
inventory, independently hashed all 3,710 referenced stdout/stderr streams,
and found zero missing, mismatched, duplicate, or orphan stream.  Twenty-five
historical result records say `orphan_cleanup_required=true`; this is evidence
that their per-run cgroups required cleanup, not that residue survived capture.
The independently hashed outer `cgroup-residue.txt` and `solver-units.txt` are
both empty, and `processes.txt` contains no batch, Umlaut, or Vampire process.
All 14 session records are present.  Package maintenance remains bound to
quiescence record SHA-256
`b3001f3770b878f1f2a067316fa3e2897318862a16c3932ddd8f4e58160f8877`.

Only after `checkpoint_verified` did the controller delete Linode `102354657`
and firewall `108358859`.  A fresh read-only provider query returns
`active: null, parked: []`.  The self-cleaning recovery task is absent,
controller PID `16036` is absent, and its launch log ends with
`controller_invocation_completed` followed by `task_launch_completed`.
The inspect-only campaign gate independently selects outer/next release `j13`
at exactly 1,855 results, leaving 845 J13 and all 5,802 CASC-2025 records.

Conclusion: the no-restart ownership transfer produced a complete, internally
consistent, quiescent, independently reproducible successor checkpoint and
safe provider teardown.  This accepts only the recovered slice.  The matrix
Bead remains in progress because 6,647 of 8,502 records are still missing; a
future guarded slice must continue J13 before any CASC-2025 transition.

## Active successor from the 1,855-result checkpoint

After the validated checkpoint evidence was pushed at root commit `5e20f577`,
the auto planner selected J13 at exactly 1,855/2,700 results.  Ignored plan
`.artifacts/casc-benchmark/j13-1855-next-resume-plan-260806.json` has SHA-256
`c06a375acda274839ae501f7f2b877cd7639762d94b12f2c2d1b475db1677ba4`.
It observed 252,000 remaining monthly allowance seconds at
`2026-08-06T07:30:24Z`, exceeding the complete 14,700-second launch guard, and
specified the exact validated archive, SHA-256, initial count 1,855, and
14,400-second batch wall.

The hidden, limited-interactive task
`Umlaut-CASC-J13-Resume-20260806T073524Z` triggered once at `07:35:24Z`,
disabled itself before invoking the controller, and revalidated the plan hash.
Launch log
`.artifacts/casc-benchmark/scheduled-launch-j13-20260806T073525Z-11180.log`
and controller log
`.artifacts/casc-benchmark/j13-resume-controller-20260806T073545Z-11180.log`
preserve the handoff.  The controller independently rechecked 252,000 seconds
of allowance before provider contact and provisioned exactly one successor:
runner `260806-073551-6bd0`, Linode `102365031`, firewall `108778464`, on
`g7-highmem-8` in `us-ord`.  Its unattended-maintenance quiescence record has
SHA-256
`ace42ca792260ccde8c045ca233be6265f721033fbaaf31ae7c0e8857740758b`.

The 4,184-file clean-main source snapshot has SHA-256
`25b62d1b10bf543b68e9b37987e2d341ae495ddd9afccdf51dfc27d02a8533d6`.
The controller uploaded and hash-verified that snapshot, the exact CASC-J13
corpus, checkpoint
`c9f54dadfab3e28f95e1d6d0fd8b16d474e0dbc5d6ac6d27dc6dc00cc313d012`,
frozen Umlaut `4e87dac3`, and pinned Vampire `5.0.1`.  Safe extraction verified
1,350 problems and 2,438 axioms.  All outer checkpoint members and the nested
archive verified, the restored summary/contract/result inventory reconciled
exactly to 1,855, and the immutable contract plus strict cgroup-v2 preflight
passed.

At `2026-08-06T07:46:12Z`, service
`casc-j13-v2-resume-260806-073551-6bd0.service` started with MainPID `3991`,
invocation `7fcddd3bf79048408785da9a713e2ba0`, and zero restarts.  A second exact
identity read remained loaded and active/running with the same PID and
invocation, `NRestarts=0`, and the untouched 1,855-result starting inventory.
This falsifies a partial restore, duplicate restart, or preflight bypass at the
handoff.  It does not accept the new slice: the scheduled task and controller
retain sole ownership until terminal capture, independent validation,
zero-residue evidence, and provider teardown complete.  The first ordinary
controller poll retained the exact service identity and advanced the inventory
to 1,858, confirming useful work without a restart.

## Verified successor J13 checkpoint at 2,352 results

Question: did the guarded 1,855-result successor survive controller transport
failures without restarting its solver service, produce a fully reproducible
checkpoint, and delete only its exact managed provider resources after local
verification?  The answer is yes.  Service
`casc-j13-v2-resume-260806-073551-6bd0.service` retained MainPID `3991`,
invocation `7fcddd3bf79048408785da9a713e2ba0`, and `NRestarts=0` throughout
the slice.  Its terminal journal binds those identities plus boot
`b18fe314fb03404593260e2c33cc4397` and success sequence `18680` to 1,855
resumed plus 497 new results, or 2,352 total.  It finished with
`Result=success`, `ExecMainStatus=0`, and inactive/dead state; the transient
unit had unloaded before the final poll, so the controller required this exact
journal identity rather than accepting an anonymous missing unit.

Transport recovery did not weaken that identity.  The original controller
failed closed after a client-address change and retained runner
`260806-073551-6bd0`, Linode `102365031`, and firewall `108778464`.  After the
firewall source was refreshed to `73.145.241.253/32`, the self-disabling hidden
task `Umlaut-CASC-J13-Recovery-20260806T095050Z` required clean synchronized
`main` at `1a49c44a95da46dd4a5c56091af742bf19eeb8bf`, seven exact critical
input hashes, and the original runner identities before every controller
invocation.  Recovery controller PID `7332` adopted the unchanged service at
2,244 results, PID `20284` re-adopted it at 2,308 after a later result-count
SSH timeout, and PID `11080` finally re-adopted it at 2,334 after a separate
status-probe timeout.  Before that final adoption, one bounded read-only probe
returned MainPID `3991`, the same invocation, zero restarts, active/running
state, and 2,333 results.  Every failed controller logged
`runner_retained_for_recovery`; none restarted the service, captured a partial
checkpoint, or deleted provider resources.

Terminal capture removed zero incomplete result artifacts and regenerated the
J13 and combined reports at exactly 2,352 records.  The ignored archive
`.artifacts/casc-benchmark/j13-checkpoint-260806-073551-6bd0.tar.gz` is
35,096,261 bytes with SHA-256
`e8cbbf65825ea70ef7da0069774af4e8b349c0619731c6fe98d078a61cf8a415`.
Its 13 regular outer members bind a unique sorted 2,352-result inventory,
SHA-256
`1edd4bee709492019db8fa74860324eddbd26342ff5d559fa3302a722a94787b`.
The nested archive is 51,382,178 bytes with 7,144 regular members and SHA-256
`39ffccdaaec5768aa010c3391ce1677ef1b209ecf311ee709a8baa5920f23226`.
The captured process snapshot and service-property evidence hash respectively
to `d450eb1d116c21ffabbfb0622ea2d5fed76122bcf935caae9d673c82eb5534b9`
and `7da19ec1655ec6c1bb73446a04af7b75bec549e3e5f8d3f5d46eca0295e49e2a`.

Strict independent validation reproduced contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`,
contract-file SHA-256
`4a66c48124cdfb89da5c17ac87229e599ae2dffd92976c0ff89804d362bc6075`,
manifest SHA-256
`939f8d03f0ceb0cbccd6377a01b605d84adeaa46e892a630513cccb82c825941`,
1,176 Umlaut plus 1,176 Vampire records, and 2,352/2,700 J13 results.  It
also reproduced the embedded combined boundary as 0/5,802 CASC-2025 plus
2,352/2,700 CASC-J13, or 2,352/8,502 total, with all 66 official CSVs kept
contextual.  J13 summary SHA-256 is
`27c872ebd031f68b7cd5178eefbde734ed3686d4167e188b10d8f3b2b5cfa470`;
combined summary SHA-256 is
`89c984009d76f43b98ca9479c6260e15fa2e08cd7779e38c124e9929d29f7e6a`.
The controller and separately invoked validation sidecars are byte-identical,
2,328 bytes, with SHA-256
`f77ee9985583156af1f7850ea6156afc464ac00bb8467c92486302c13f0efedd`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/j13-checkpoint-260806-073551-6bd0.tar.gz `
  --archive-sha256 e8cbbf65825ea70ef7da0069774af4e8b349c0619731c6fe98d078a61cf8a415 `
  --manifest benchmarks/casc_2026_manifest.jsonl `
  --run-name casc-j13-2026-089e06c8-v2 `
  --contract-id 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --expected-results 2352 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/j13-checkpoint-260806-073551-6bd0.tar.gz.independent-validation.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260806-073551-6bd0.tar.gz `
  --checkpoint-sha256 e8cbbf65825ea70ef7da0069774af4e8b349c0619731c6fe98d078a61cf8a415 `
  --inspect-only `
  --output .artifacts/casc-benchmark/j13-2352-inspect-260806.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
```

Inspect-only output SHA-256
`9c314f528e1902ee461be28ed3b6065c4fe00515c830d8804e5a265bbca39baa`
selects outer and next release `j13` at exactly 2,352 results and status
`resume_candidate`.  Only after `checkpoint_verified` did the controller
delete Linode `102365031` and firewall `108778464`.  Fresh local and provider
checks return `active: null`, `parked: []`, and zero restricted-reaper runners.
The recovery task is disabled and its last result is zero; the launch log ends
with `controller_invocation_completed` and `task_launch_completed`.

Conclusion: the recovered successor is a complete, quiescent, independently
reproducible checkpoint with safe provider teardown.  This accepts only this
slice.  J13 still lacks 348 records, CASC-2025 still lacks all 5,802, and the
matrix Bead remains in progress with 6,150 of 8,502 records outstanding.  A
future guarded slice must continue J13 from this exact checkpoint before any
CASC-2025 transition.

## Partial-checkpoint Umlaut failure audit

Question: do the 77 Umlaut `error`/`crash` classifications in the independently
validated 1,855-result checkpoint represent only the already fixed THF parser
cohort, or do they expose distinct follow-up defects?  The exact checkpoint
archive and nested run were extracted locally under
`C:\tmp\umlaut-j13-analysis-260806-0809`.  PowerShell parsed all 928 completed
Umlaut result JSON files, grouped non-success records by independently recorded
stderr SHA-256, read the referenced streams, and reconciled the groups to the
summary's 76 errors plus one crash.

Seventy-three exit-3 records have identical stderr SHA-256
`9c8ffe4f988b015afb5af4f2652f14a13055ebff43ae502cd5c09cefa776caee`
and `Too many arguments applied to the term`.  They exactly match the 73-input
frozen-binary cohort already repaired and comprehensively validated by
`E_Rust_Port-9jt.2.11`; they are immutable historical results, not a regression
in current `main`.  One frozen-binary exit-4 record, `ITP185^1`, was also
accepted by that Bead's final current-main 398/400 audit.

The remaining two errors are `SYO544^1` and `SYO545^1`, with identical stderr
SHA-256
`83ba21216b656fa5a8d845d022050ff8696547761c6cf578eac2d78f4bd603ba`
and `Boolean formula equality requires Boolean right operand`.  They are the
same two inputs still rejected by the final current-main THF audit.  Both
compare values of complete sort `$o > $o`, including equality against a
constant-false lambda and negation, and both are solved by multiple official
J13 entrants.
This distinct type-dispatch gap is now tracked by bug
`E_Rust_Port-9jt.2.14` with a 400/400 THF acceptance boundary.
The hash-bound proving probe now also accepts an explicit audit classification
and exact expected selection count.  After runner teardown it can select the
two `error` records from immutable audit
`8457b397f123fef0bb8149acc9fbcceb1a4d8568f57bbcd6eeb64a6e1477beb7`
and fail unless both repaired inputs enter production proving; its historical
default remains the 73 `too_many_arguments` records.  Four focused selection
tests cover multi-class audit-order preservation, exact-count rejection,
duplicate rejection, and an empty requested class; the local no-prover smoke
still selects exactly the two production errors and all 73 historical
overapplication records.

The crash is new: result
`casc-runs/casc-j13-2026-089e06c8-v2/results/umlaut/fnq/0925-fnq-e124691e4d3d.json`
binds `HWV063+1`, input SHA-256
`dc8bd86b5fa21351ee69b677f051699866d25c1aa0ab848d0b4fe016bfc234b3`,
return code `-6`, 0.068215 CPU seconds, 0.261169348 wall seconds, 22.359375 MiB
peak memory, no cgroup memory event, no cleanup requirement, empty stdout, and
stderr SHA-256
`ab030f25249d0efcdbccacf45bf97dc0385148a3227420c19eaa1a62f0baacd8`.
The referenced stderr says the Rust main thread overflowed its stack and then
aborted.  Exact source analysis found 4,148,434 bytes and 182,709 lines, three
quantifier lists containing 332, 4, and 32,896 variables, 77,655 conjunctions,
105,044 disjunctions, but only four levels of textual parenthesis nesting.
That falsifies a cgroup/resource-limit classification and literal deep source
nesting; it does not yet identify which recursive parser or AST traversal
overflows.  Bug `E_Rust_Port-9jt.2.13` requires a clean current-main Ubuntu
reproduction, root-cause isolation, an iterative or explicitly bounded fix,
and a compact synthetic regression rather than a larger process stack.

Representative reproduction from the repository root:

```powershell
tar.exe -xzf `
  .artifacts/casc-benchmark/j13-checkpoint-260806-030618-c092.tar.gz `
  -C C:\tmp\umlaut-j13-analysis-260806-0809 `
  j13-checkpoint-260806-030618-c092/casc-runs.tar.gz
tar.exe -xzf `
  C:\tmp\umlaut-j13-analysis-260806-0809\j13-checkpoint-260806-030618-c092\casc-runs.tar.gz `
  -C C:\tmp\umlaut-j13-analysis-260806-0809 `
  casc-runs/casc-j13-2026-089e06c8-v2
Get-FileHash -Algorithm SHA256 `
  'problems/casc_2026/FNQ/HWV063+1.p'
rg -n 'SYO544\^1|SYO545\^1' cast_2026_results
```

### Current-main static hardening draft

Question: can the exact structural evidence identify guaranteed stack hazards
without touching the runner owned by the active successor?  A represented-TSTP
call-path audit found two such hazards.  The variable-list parsers formerly
recurred once for every comma-separated binder, so the inner 32,896-variable
list necessarily consumed 32,896 Rust frames before parsing the matrix.  The
parser then built the 77,656-operand conjunction as a left-associated binary
tree, and the first represented formula preprocessing passes recursively
descended both that tree and the complete quantifier prefix.

The implemented working-tree change addresses the complete exact-input
route rather than only the first observed recursion.  TSTP and old-TPTP binder
lists are parsed in a loop with one lexical variable environment and scoped
declaration per binder, then reverse-wrapped into the same quantifier order.
This restores names after both successful and failed parses and makes repeated
same-sort names shadow rather than alias.  Associative
TSTP conjunctions and disjunctions remain left-associated through 1,024
operands for ordinary compatibility, while larger ordered chains are built as
deterministic balanced trees.  Free-variable collection uses an explicit
visit/leave stack with binder-identity counts.  Boolean-equality replacement,
literal expansion, FOOL unrolling, simplification, definition discovery and
copying, polarity marking, NNF, miniscope scanning and bound-aware copying,
variable renaming, and Skolemization now peel and rebuild contiguous
first-order quantifier prefixes iteratively.  Named-lambda renaming retains its
existing single-binder copy behavior.

Compact regressions exercise 16,384 binders under the default test-thread
stack, a 16,384-operand associative chain whose connective depth is at most
14, malformed-list environment cleanup, same-name shadowing, the core formula
traversals, and Boolean-equality replacement.  Static inspection also found
that exact HWV063+1 loses its 33,228 existential binders during Skolemization;
only four universal binders reach later quantifier-shifting/CNF recursion, and
the balanced matrix has logarithmic connective depth.  This falsifies the
hypothesis that changing only the parser recursion is a sufficient fix, while
providing a bounded reason the later untouched CNF stages should be safe.

Reproducible local inspection commands (no Rust execution) are:

```powershell
Get-FileHash -Algorithm SHA256 `
  'problems/casc_2026/FNQ/HWV063+1.p'
rg -n `
  'parse_quantified_tformula|tformula_collect_free_vars|tformula_rek_skolemize' `
  src/terms/termbanks.rs src/clauses/clausefunc.rs
git diff --check
bd show E_Rust_Port-9jt.2.13 --json
```

### Ubuntu acceptance evidence

The verified 2,352-result checkpoint freed the ordinary Ubuntu runner for
acceptance without touching the completed high-memory campaign provider.  The
final source content was uploaded from base commit `8ae429bf231583cbacf8669272805db3b6bff2ee`
as 4,185 files; the final standalone snapshot archive SHA-256 is
`3b5ed138e04562a3031fc4c5ac10b47d1d4864b7be4531274307c2a265120356`.
The only changes after the exact THF and HWV runs were two targeted Clippy
allowances and one panic-contract documentation paragraph, so their executable
formula behavior is identical to the finally lint-clean source.

Strict THF validation accepted all 400 untouched J13 THF inputs.  Audit JSON
`.artifacts/casc-benchmark/j13-thf-syntax-after-equality-56454706.json` has
SHA-256 `c56ad8283a75f1ceed8f99f12f672d7cf2e2a050ad2358cd6b36a83e01b552b8`.
The production proving probe selected exactly `SYO544^1` and `SYO545^1`; both
entered proving, and its JSON has SHA-256
`8e0e5482e8080e0b43684473d074982f71e1699b442b378d42db61157c0227bc`.
Four pure-Python probe regressions also pass.  This closes the complete 400/400
boundary for `E_Rust_Port-9jt.2.14` while retaining exact mixed-arrow/scalar
sort rejection.

Exact `HWV063+1` ran as guarded systemd service
`umlaut-hwv063-56454706.service`, MainPID `49192`, invocation
`b43bb3caedb3408bb31c002cc74df208`, and `NRestarts=0`.  The 180.308445811-second
wall guard deliberately terminated proving after 258.309013 CPU seconds at
1,997,324,288 bytes (1,904.796875 MiB) peak memory.  All cgroup memory events
were zero, no orphan cleanup was required, and neither stack-overflow nor abort
text occurred.  Result JSON SHA-256 is
`f50a714ea1c4084e35fd9f2882bee35dffb8c065b4179790f0259763ce4e48ed`;
the complete ignored evidence archive
`.artifacts/casc-benchmark/hwv063-56454706.tar.gz` has SHA-256
`4988134fc4a0a283916924ac1f1e9e23d8bba5b9617192a926cd709e6dd193ec`.
This closes the exact-input boundary for `E_Rust_Port-9jt.2.13` without raising
the process stack.

The final comprehensive Ubuntu evidence is intentionally composite because the
outer client guard closed its stdout while the last benchmark was still
running.  Workload `260806-133651-af34` passed rustfmt, strict
`cargo clippy --locked --all-targets --all-features -- -D warnings -D
clippy::pedantic`, all 4,575 library tests plus every binary/integration target,
debug and release builds, the Windows GNU test/release cross-builds, all
solution-validator tests, the 50-case main comparison with zero mismatches, and
the 216-case tool comparison with zero mismatches.  Its preserved bundle
`.artifacts/casc-benchmark/j13-acceptance-260806-133651-af34.tar.gz` has SHA-256
`0949f966a7bb9ef958c53069077430227b92e625bc8c740f69d3c6103bf38237`.
The client-induced Python exit 120 occurred only after benchmark case 7/10;
the same unchanged source was resynchronized, rebuilt to release binary
SHA-256 `98cba296a43f7fd1b67d3542c8f10b3e5e3fac65535cf2fcc4c7bbab30793c1b`,
and rerun with output captured remotely.  All 10 cases and five repetitions
completed with zero behavior mismatches, aggregate Rust/C wall ratio
`1.0899385623121118`, and `regression_over_threshold: false`.  The final
benchmark archive, including JSON and CSV samples, has SHA-256
`901009a135668c30829f80c0b1caf83740f77b9ee10e79a16170db66fa4230cb`.

Two rejected attempts remain useful negative evidence.  Workload
`260806-125213-2e95` exposed 12 compatibility regressions that led to restoring
external-name persistence plus old-TPTP `$let` scalar-sort recovery.  Workload
`260806-133055-2136` exposed a load-sensitive LTB child-output failure after the
core suite; the exact isolated test then passed in 0.48 seconds, and the next
comprehensive workload passed it in 0.54 seconds.  No failing attempt was used
as acceptance evidence.  The ordinary runner finished parked with no active
lease and its remote reaper armed.

Conclusion: the immutable partial matrix separated historical frozen-binary
failures from two actionable current gaps, and both gaps now have focused,
exact-input, and comprehensive acceptance evidence.  This does not complete
the campaign: J13 still lacks 348 records and must resume from the verified
2,352-result checkpoint before CASC-2025 starts.

## Cost-bound teardown after the 2,416-result transport failure

Question: could the retained J13 runner be salvaged without crossing another
rounded billing hour, and how is the same failure now prevented from retaining
a paid runner indefinitely?  The controller last verified service
`casc-j13-v2-resume-260806-143950-781d.service` at 2,416/2,700 results,
MainPID `3968`, invocation `6bb6df0701a54abf92c97af9bdaaaea5`, and zero
restarts.  The next exact `systemctl show` probe timed out after the client
address changed, so the old controller failed closed and retained run
`260806-143950-781d`.

The recovery attempt began inside the five-minute guard before the next
Linode-provided hourly boundary.  There was no longer enough time to arm a
15-minute salvage lease.  Following the preregistered cost-first rule, no SSH
refresh, solver restart, or unverified capture was attempted.  The exact saved
Linode `102384868` and firewall `109420354` were deleted at
`2026-08-07T02:36:22Z`, before the `02:40:20Z` boundary.  Provider `status`
then reported `active: null` and no parked runners; a dry `gc` reported no
stale managed resources.  The last authoritative archive therefore remains
the independently validated 2,352-result checkpoint with SHA-256
`e8cbbf65825ea70ef7da0069774af4e8b349c0619731c6fe98d078a61cf8a415`.

The permanent controller now replaces unbounded retention with
`linode-runner guard-recovery --grace-seconds 900`.  It revalidates exact live
resource identity, refreshes only the saved firewall `/32`, preserves the
workspace, and sets a non-extendable deadline at the earlier of 15 minutes or
the current paid-hour cutoff.  Both the restricted remote systemd reaper and
the hidden Windows Scheduled Task are required.  Failure to validate or arm
the guard falls back to immediate exact deletion, and new acquisition remains
blocked while lifecycle `guarded-recovery` exists.  Immediate deletion also
reconciles restricted reaper access for any unrelated parked leases.

Validation passed 110 focused Linode/CASC controller tests (five environment
skips), 60 complete CASC experiment tests including real synthetic Task
Scheduler cases, Python compilation, PowerShell parsing, and `git diff
--check`.  The campaign remains incomplete by design; this incident neither
advances the validated count beyond 2,352 nor closes the matrix Bead.

## Automatic controller-IP recovery

Question: can a long-running controller survive a change to its public IPv4
without weakening the SSH firewall or risking duplicate workload commands?
The prior cost guard bounded retention after a transport failure, but ordinary
SSH and SCP calls still trusted the `/32` saved at provisioning time.

The Linode runner transport now performs a silent public-IPv4 preflight before
each managed SSH or SCP connection.  If the observed `/32` differs, it takes
the lifecycle lock, requires the caller to match the exact saved run, label,
Linode ID, firewall ID, and Linode IPv4, validates both live provider resources
and their managed labels/status, then replaces only the exact firewall rules
and atomically persists `allow_cidr` plus `firewall_refreshed_at`.  An outage of
the public-IP discovery service does not break an already-authorized
connection; a forced repair fails closed because it cannot safely guess an
address.  Unchanged addresses make no provider calls.

The CASC controller marks only its bounded, read-only service and inventory
probes with `linode-runner exec --retry-safe`.  A timeout or SSH transport exit
255 then causes one forced exact firewall reconciliation and one repeat after
a two-second propagation delay.  General `exec`, solver launch, capture,
upload, and download operations remain non-retrying, so an ambiguous
disconnect cannot execute a state-changing command twice.  The PowerShell
wrapper preserves `--retry-safe` on the controller side of its base64 command
boundary.

Reproduction commands from the repository root were:

```powershell
.\.venv\Scripts\python.exe -m unittest discover `
  -s tools\linode-runner -p test_*.py
.\.venv\Scripts\python.exe -m unittest discover `
  -s experiments\2026-07-28-004-casc-benchmark-matrix -p test_*.py
.\.venv\Scripts\python.exe -m py_compile `
  tools\linode-runner\linode_runner.py `
  tools\linode-runner\test_linode_runner.py `
  experiments\2026-07-28-004-casc-benchmark-matrix\test_resume_j13_checkpoint.py
$files = @(
  'linode-runner.ps1',
  'experiments\2026-07-28-004-casc-benchmark-matrix\resume_j13_checkpoint.ps1'
)
foreach ($file in $files) {
  $tokens = $null
  $errors = $null
  [System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path $file), [ref]$tokens, [ref]$errors
  ) | Out-Null
  if ($errors.Count -gt 0) { throw ($errors -join "`n") }
}
git diff --check
.\linode-runner.ps1 status
.\linode-runner.ps1 gc --older-than-hours 1
```

All 162 Linode-runner tests passed with six platform/dependency skips, and all
60 CASC controller, planner, scheduler, report, and validator tests passed.
Regressions falsify stale local identity, mismatched live identity, unnecessary
provider writes for an unchanged address, public-IP discovery failure, wrapper
option corruption, and unsafe retry of a timed-out mutating command.  The exact
incident shape—a retry-safe 90-second status timeout—forces one firewall refresh
and then succeeds on its single repeat.  Python compilation, both PowerShell
parsers, and `git diff --check` passed.  Final read-only provider evidence was
`active: null`, no parked runners, and no stale managed resources.

Conclusion: controller public-IP drift is now repaired before the next normal
transport connection, while the guarded-recovery reapers remain the independent
cost backstop.  This removes the observed `/32` failure mode but cannot make an
internet connection infallible; a sustained network or provider outage still
enters the existing bounded recovery/deletion path.  No runner was provisioned
and the authoritative campaign count remains 2,352.

## Verified guarded J13 checkpoint at 2,521 results

Question: did the first production slice using automatic controller-IP repair
preserve the exact solver-service identity, produce a fully reproducible
successor checkpoint, and delete only its exact managed provider resources?
The answer is yes.  The authorized plan
`.artifacts/casc-benchmark/j13-2352-next-resume-plan-260807T033910Z.json`,
SHA-256
`4e501493f8cf55fd2a2773779c4d85325df695a58e256f96bc7d333c4e3e4fc5`,
launched runner `260807-034544-4a12`, label
`e-rust-codex-260807-034544-4a12`, Linode `102413430`, and firewall
`110411448`.  The uploaded source archive SHA-256 was
`f82b31eabfd730e37f2a809c2e4086fcc42c87f216f654b156bcb71c20f3f64e`;
package-maintenance quiescence remained bound to SHA-256
`287c6d8a6044f59732b8d40f9ffaea8f4ec8a49e34dd059f4e9143c309de80a2`.
All checkpoint, corpus, binary, contract, cgroup-v2, and maintenance preflights
passed before service `casc-j13-v2-resume-260807-034544-4a12.service`
started with MainPID `3997`, invocation
`ec9f3f1e75ac4741b17f8e395a49fa24`, and zero restarts.

The service retained those identities throughout the slice.  Its terminal
journal has one boot, `ba12f134b45d4b09bc1535eb25ffe188`, and one
invocation.  Sequence `19886` reports contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`
with 2,352 resumed plus 169 new records; sequence `19887` is the unique
systemd success record.  Final properties are `Result=success`,
`ExecMainStatus=0`, `NRestarts=0`, and inactive/dead.  Terminal capture removed
zero incomplete artifact pairs and regenerated the partial J13 and combined
reports at exactly 2,521 records.

The ignored archive is
`.artifacts/casc-benchmark/j13-checkpoint-260807-034544-4a12.tar.gz`:
36,459,474 bytes with SHA-256
`94d6591ba7af1062fe929c8ddd25d358a98d73aee68827b4ab69bf6ddfd1c50e`.
Its 13 regular outer members bind 12 hashes and a unique, sorted 2,521-path
result inventory, SHA-256
`a60878d4985a5b637a8acd68bbf171fd74d177d60b8380c9e796e12b83e38888`.
The nested archive is 52,780,339 bytes with 7,653 regular members and SHA-256
`07b128e4b51f0534c3bcfe3866f42e0a3f91d57d6ce233151a03922994611122`.
Captured process and service-property evidence hash respectively to
`a56f8f5a66b7c9df4a82d07c0a05dd977f472cf1c2b288a8032578475d3f6ae5`
and `7f14472be325fab8015b9264197c43bdbdcf2fc2e3285b80394331b63f27c638`.

Strict independent validation reproduced contract-file SHA-256
`4a66c48124cdfb89da5c17ac87229e599ae2dffd92976c0ff89804d362bc6075`,
manifest SHA-256
`939f8d03f0ceb0cbccd6377a01b605d84adeaa46e892a630513cccb82c825941`,
1,261 Umlaut plus 1,260 Vampire records, and 2,521/2,700 J13 results.  It
also reproduced the embedded combined boundary as 0/5,802 CASC-2025 plus
2,521/2,700 CASC-J13, or 2,521/8,502 total, with all 66 official CSVs
contextual and every report acceptance surface present.  J13 summary SHA-256
is `f35e0bc47429881c8be0b4607bc994f06d12ad258df52c0eaf3412aee2bc34cb`;
combined summary SHA-256 is
`4d813eb9ac1967e74da69ac6c68d8f04152646552d02237f26c2e7c4ec95b63b`.
Controller and independent validation sidecars are byte-identical, 2,328
bytes, with SHA-256
`b5e67312e995493180afed3c6c067c3c91648f06d1eb75e0d98bb6740365fe48`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/j13-checkpoint-260807-034544-4a12.tar.gz `
  --archive-sha256 94d6591ba7af1062fe929c8ddd25d358a98d73aee68827b4ab69bf6ddfd1c50e `
  --manifest benchmarks/casc_2026_manifest.jsonl `
  --run-name casc-j13-2026-089e06c8-v2 `
  --contract-id 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --expected-results 2521 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/j13-checkpoint-260807-034544-4a12.tar.gz.independent-validation.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260807-034544-4a12.tar.gz `
  --checkpoint-sha256 94d6591ba7af1062fe929c8ddd25d358a98d73aee68827b4ab69bf6ddfd1c50e `
  --inspect-only `
  --output .artifacts/casc-benchmark/j13-2521-inspect-260807.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

As an independent falsification check, the archive was extracted under
`C:\tmp\umlaut-casc-audit-260807-034544-4a12`.  PowerShell verified all 12
outer hashes, exact agreement between the outer inventory and 2,521 result
JSON paths, 2,521 unique solver/problem coordinates, and all 5,042 referenced
stdout/stderr streams by content hash.  It found zero missing, mismatched,
duplicate, or orphan streams.  All 18 session records are present; 34
historical results report `orphan_cleanup_required=true`, which records
successful per-run cleanup rather than surviving residue.  The capture has
174 single-boot/single-invocation journal records, one terminal success record,
no standalone batch/Umlaut/Vampire process, and empty `cgroup-residue.txt` and
`solver-units.txt`.

The controller verified the archive before deleting exact Linode `102413430`
and firewall `110411448`, then recorded `managed_resources_deleted`.  Fresh
runner status is `active: null` with no parked runner; provider check reports
zero restricted-reaper resources; dry GC finds no stale managed resource.  The
self-disabled task `Umlaut-CASC-J13-Resume-20260807T034448Z` has last result
zero, and its launch log ends with `controller_invocation_completed` followed
by `task_launch_completed`.

Inspect-only output
`.artifacts/casc-benchmark/j13-2521-inspect-260807.json`, SHA-256
`8dc5e6c9b4a47c61992a3e1b8a4f73cf82ecf8601dd1fedc31d47342e001c38e`,
selects outer and next release `j13` at exactly 2,521 results with status
`resume_candidate`.  Fresh nonmutating successor plan
`.artifacts/casc-benchmark/j13-2521-next-resume-plan-260807T0807Z.json`,
SHA-256
`0f97d83278749a6e658f231ba110a0e7984b855d4c5037a69890173905b76de4`,
observed 172,800 allowance seconds at `2026-08-07T08:08:27Z` against a
14,700-second service requirement (18,000 billed seconds) and reports
`ready_to_arm`.  It binds release `j13`, this exact checkpoint and hash,
initial count 2,521, and a 14,400-second batch wall.  No scheduled task or
runner was created.

Conclusion: this is a complete, quiescent, independently reproducible
successor with safe provider teardown.  It accepts only this slice.  J13 still
lacks 179 records, CASC-2025 still lacks all 5,802, and the matrix Bead remains
in progress with 5,981/8,502 records outstanding.  The next guarded slice must
continue J13 from this exact checkpoint before any CASC-2025 transition.

## Verified guarded J13 checkpoint at 2,645 results

Question: did the second production slice using automatic controller-IP
recovery preserve the exact solver-service identity, produce a complete and
independently reproducible successor to the 2,521-result checkpoint, and
delete only its exact managed provider resources?  The answer is yes.  The
authorized plan
`.artifacts/casc-benchmark/j13-2521-next-resume-plan-260807T0807Z.json`,
SHA-256
`0f97d83278749a6e658f231ba110a0e7984b855d4c5037a69890173905b76de4`,
launched runner `260807-081357-0185`, label
`e-rust-codex-260807-081357-0185`, Linode `102423287`, and firewall
`110754836`.  The uploaded source snapshot SHA-256 was
`39833e8af81b3f0ec3156fe7478faa457a8aeb444e8505c1c69c56e5f96231ba`;
package-maintenance quiescence remained bound to SHA-256
`5e9b51261607d54c9513741ebd5052fbff88335d13d9dc44b63b8fdce99ea004`.
All checkpoint members, the exact 2,521-result inventory and summary, corpus,
binary, contract, cgroup-v2, and maintenance preflights passed before service
`casc-j13-v2-resume-260807-081357-0185.service` started with MainPID `3986`,
invocation `a5a84a4bdeb04fea9be348646a76f82e`, and zero restarts.

The service retained those identities throughout the slice.  Its terminal
journal has one boot, `0847383c27864b5a971e48c75e782b89`, and one
invocation.  Sequence `19881` reports contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`
with 2,521 resumed plus 124 new records; sequence `19882` is the unique
systemd success record.  Final properties are `Result=success`,
`ExecMainStatus=0`, `NRestarts=0`, and inactive/dead.  Terminal capture found
zero incomplete artifact pairs and regenerated the partial J13 and combined
reports at exactly 2,645 records.

The ignored archive is
`.artifacts/casc-benchmark/j13-checkpoint-260807-081357-0185.tar.gz`:
37,295,815 bytes with SHA-256
`87f3dddc18262c0c04342a0a0822f49574705473a0e67c638b1ae84c8af645d3`.
Its 13 regular outer members bind 12 hashes and a unique, sorted 2,645-path
result inventory, SHA-256
`9f78a9b24e54c8e19f76788d9b43ce4e50a8f265308d9ee9c186b3e67f80f18f`.
The nested archive is 53,642,022 bytes with 8,027 regular members and SHA-256
`91e0a9db92f7d661c68031ff9f6ba28de09dc56edaa79477982ea5b39cbecf05`.
Captured process and service-property evidence hash respectively to
`2aeaf1677f1f4f7ecfb257e8db73c9c37db949414550126c8c8e875e88fd2ded`
and `6bfbabbcf60c30e4138594e2fc8bf7860019635ad5be4f67a91e3636cb54d3bb`.

Strict independent validation reproduced contract-file SHA-256
`4a66c48124cdfb89da5c17ac87229e599ae2dffd92976c0ff89804d362bc6075`,
manifest SHA-256
`939f8d03f0ceb0cbccd6377a01b605d84adeaa46e892a630513cccb82c825941`,
1,323 Umlaut plus 1,322 Vampire records, and 2,645/2,700 J13 results.  It
also reproduced the embedded combined boundary as 0/5,802 CASC-2025 plus
2,645/2,700 CASC-J13, or 2,645/8,502 total, with all 66 official CSVs
contextual and every report acceptance surface present.  J13 summary SHA-256
is `dafcdeb7a385f21abb27931b30fed9fa05b5e71a067af15600c128fa4311cb65`;
combined summary SHA-256 is
`c55db6c5510ffe6181b78800406cbe4bef03d98435065b4829dc79a1d31fadd8`.
Controller and independent validation sidecars are byte-identical with
SHA-256
`d3c3fc6188a150d52379c327de36db13989dc45b21fffd721abc7d8a7440f575`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/j13-checkpoint-260807-081357-0185.tar.gz `
  --archive-sha256 87f3dddc18262c0c04342a0a0822f49574705473a0e67c638b1ae84c8af645d3 `
  --manifest benchmarks/casc_2026_manifest.jsonl `
  --run-name casc-j13-2026-089e06c8-v2 `
  --contract-id 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --expected-results 2645 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/j13-checkpoint-260807-081357-0185.tar.gz.independent-validation.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260807-081357-0185.tar.gz `
  --checkpoint-sha256 87f3dddc18262c0c04342a0a0822f49574705473a0e67c638b1ae84c8af645d3 `
  --inspect-only `
  --output .artifacts/casc-benchmark/j13-2645-next-resume-inspection-260807T1230Z.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\j13-checkpoint-260807-081357-0185.tar.gz `
  -ArchiveSha256 87f3dddc18262c0c04342a0a0822f49574705473a0e67c638b1ae84c8af645d3 `
  -RunName casc-j13-2026-089e06c8-v2 `
  -ExpectedResults 2645 `
  -ContractId 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  -ExpectedNewResults 124 `
  -ExpectedResumedResults 2521 `
  -ExpectedContractSequence 19881 `
  -ExpectedSuccessSequence 19882 `
  -ExpectedBootId 0847383c27864b5a971e48c75e782b89 `
  -ExpectedInvocationId a5a84a4bdeb04fea9be348646a76f82e `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-script4-260807-081357-0185 `
  -Output .artifacts\casc-benchmark\j13-checkpoint-260807-081357-0185.tar.gz.raw-audit.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The new experiment-local `audit_casc_checkpoint.ps1` makes the independent
raw audit reusable.  It verifies archive path safety and SHA-256 before
extraction, every outer member hash, exact sorted result inventory, unique
solver/problem coordinates, every referenced stream hash, session and journal
identity, terminal contract and success sequences, process absence, and empty
cgroup/unit residue.  The successful audit found 2,645 coordinates, 5,290
referenced streams, zero missing/mismatched/orphan streams, 20 session records,
129 journal records, no standalone batch/Umlaut/Vampire process, and empty
residue files.  Its ignored JSON output has SHA-256
`2bdbb654cc7e65fb07934c784722a42b53b49b04a41133fdb9431cc0b5aab0c6`.
Forty-one historical results report
`orphan_cleanup_required=true`; this records successful per-run cleanup rather
than surviving residue.  As falsification checks, the auditor rejects a wrong
archive hash before creating its extraction root and uses explicit expected
boot, invocation, contract, result, and terminal-sequence values rather than
accepting whatever the archive reports.  PowerShell parsing and the complete
real-archive smoke run passed.

The controller verified the archive before deleting exact Linode `102423287`
and firewall `110754836`, then recorded `managed_resources_deleted`.  Fresh
runner status is `active: null` with no parked runner; provider check reports
zero restricted-reaper resources; dry GC finds no stale managed resource.  The
self-disabled task `Umlaut-CASC-J13-Resume-20260807T081327Z` has last result
zero, and its launch log ends with `controller_invocation_completed` followed
by `task_launch_completed`.

Inspect-only output
`.artifacts/casc-benchmark/j13-2645-next-resume-inspection-260807T1230Z.json`,
SHA-256
`8882d52a857aabc342ac9615c895c3a2afc7689d6da8ca19e665024fb49357d1`,
selects outer and next release `j13` at exactly 2,645 results with status
`resume_candidate`.  Conclusion: this is a complete, quiescent, independently
reproducible successor with safe provider teardown.  It accepts only this
slice.  J13 still lacks 55 records, CASC-2025 still lacks all 5,802, and the
matrix Bead remains in progress with 5,857/8,502 records outstanding.  The
next guarded slice must finish J13 from this exact checkpoint before any
CASC-2025 transition.

## Verified complete J13 checkpoint at 2,700 results

Question: did the final guarded J13 slice preserve the exact resumed service,
complete every J13 solver/problem coordinate, produce a reproducible terminal
checkpoint, and delete only its managed provider resources?  The answer is
yes.  Plan
`.artifacts/casc-benchmark/j13-2645-next-resume-plan-260807T2233Z.json`,
SHA-256
`ace9657e8106b75257228e25ab4940b67926b71a708529ca948049734d707a29`,
launched runner `260807-224005-66a9`, label
`e-rust-codex-260807-224005-66a9`, Linode `102456275`, and firewall
`111807443`.  The uploaded source snapshot SHA-256 was
`83eaa9f77665c9c6c86b44b7bfb8cb0056e98aac7545deb2929455d6df937050`;
package-maintenance quiescence was bound to SHA-256
`b4071f235124ae22677c80363f81025d9c552051bfae58448cb749c8f4a7b416`.
All exact 2,645-result restore, corpus, binary, contract, cgroup-v2, and
maintenance preflights passed before service
`casc-j13-v2-resume-260807-224005-66a9.service` started with MainPID `4009`,
invocation `66d266e1da554997846959895df09878`, and zero restarts.

The service retained those identities throughout the slice.  Its terminal
journal has one boot, `57d39eada2944004b8c6e7297a407b24`, and one
invocation.  Sequence `7526` reports contract
`9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676`
with 2,645 resumed plus 55 new records; sequence `7527` is the unique systemd
success record.  Final properties are `Result=success`, `ExecMainStatus=0`,
`NRestarts=0`, and inactive/dead.  Terminal capture removed zero incomplete
artifact pairs and regenerated the reports at exactly 2,700 J13 and 2,700
combined results.

The ignored archive is
`.artifacts/casc-benchmark/j13-checkpoint-260807-224005-66a9.tar.gz`:
37,691,804 bytes with SHA-256
`5eff5801c1bb3f2f0656432622afab22cd563a77fed123fdf83ac2dea25efecf`.
Its 13 regular outer members bind 12 hashes and a unique, sorted 2,700-path
result inventory, SHA-256
`38afedba1003203e191a8062b4f7481ce89d5c98d7280dc10b7bd281d668f104`.
The nested archive is 54,045,958 bytes with 8,194 regular members and SHA-256
`9caf7b3bb78f7e3d81a6752dbdea7591c9735e85acdc3f08a88e083af4afb65a`.
Captured process and service-property evidence hash respectively to
`3f16d70c620f0f697bdac6faa9b2a5f05fbf04938430fceb576cfe7d1f55f13f`
and `0d55e5464c8fbcb5650ba6d8b0f7f27ea20031ec1dec9c86d06368b7d20398ef`.

Strict independent validation reproduced contract-file SHA-256
`4a66c48124cdfb89da5c17ac87229e599ae2dffd92976c0ff89804d362bc6075`,
manifest SHA-256
`939f8d03f0ceb0cbccd6377a01b605d84adeaa46e892a630513cccb82c825941`,
1,350 Umlaut plus 1,350 Vampire records, and 2,700/2,700 J13 results.  It
also reproduced the embedded combined boundary as 0/5,802 CASC-2025 plus
2,700/2,700 CASC-J13, or 2,700/8,502 total.  J13 summary SHA-256 is
`1daeb53a366e74fcde535b40c22f1b831ce886f87a2345d38f5a6ae2768d023b`;
combined summary SHA-256 is
`3a6525f7eb535df877162781ba96477c1effe795cc59c26f866a6d9ad5e95da9`.
Controller and independent validation sidecars are byte-identical, 2,326
bytes, with SHA-256
`13d399e9ae858da12b9aa6b98637ff75b0022f84a10771c1e48597ddb5186da3`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/j13-checkpoint-260807-224005-66a9.tar.gz `
  --archive-sha256 5eff5801c1bb3f2f0656432622afab22cd563a77fed123fdf83ac2dea25efecf `
  --manifest benchmarks/casc_2026_manifest.jsonl `
  --run-name casc-j13-2026-089e06c8-v2 `
  --contract-id 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --expected-results 2700 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/j13-checkpoint-260807-224005-66a9.tar.gz.independent-validation.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\j13-checkpoint-260807-224005-66a9.tar.gz `
  -ArchiveSha256 5eff5801c1bb3f2f0656432622afab22cd563a77fed123fdf83ac2dea25efecf `
  -RunName casc-j13-2026-089e06c8-v2 `
  -ExpectedResults 2700 `
  -ContractId 9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  -ExpectedNewResults 55 `
  -ExpectedResumedResults 2645 `
  -ExpectedContractSequence 7526 `
  -ExpectedSuccessSequence 7527 `
  -ExpectedBootId 57d39eada2944004b8c6e7297a407b24 `
  -ExpectedInvocationId 66d266e1da554997846959895df09878 `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-j13-260808-0124 `
  -Output .artifacts\casc-benchmark\j13-checkpoint-260807-224005-66a9.raw-audit.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/j13-checkpoint-260807-224005-66a9.tar.gz `
  --checkpoint-sha256 5eff5801c1bb3f2f0656432622afab22cd563a77fed123fdf83ac2dea25efecf `
  --inspect-only `
  --output .artifacts/casc-benchmark/casc2025-next-inspect-260808T0124Z.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The reusable raw auditor found 2,700 coordinates, 5,400 referenced streams,
zero missing/mismatched/orphan streams, 22 session records, 60 journal
records, no standalone batch/Umlaut/Vampire process, and empty cgroup/unit
residue.  Its ignored JSON output has SHA-256
`0f4b307c5625c9b9b66bfcc6073d3eee639c2f813908481ac094dfa070f2a63d`.
Forty-five historical results report `orphan_cleanup_required=true`; this
records successful per-run cleanup rather than surviving residue.  As a
falsification check, the auditor rejected an all-zero archive hash before
creating either its fresh extraction root or output file.  Exact expected
boot, invocation, contract, result, and terminal-sequence values further
prevent the audit from accepting identities merely copied from the archive.

The controller verified the archive before deleting exact Linode `102456275`
and firewall `111807443`, then recorded `managed_resources_deleted`.  Fresh
runner status is `active: null` with no parked runner; provider check reports
zero restricted-reaper resources; dry GC finds no stale managed resource.
The self-disabled task `Umlaut-CASC-J13-Resume-20260807T223931Z` has last
result zero, and its launch log ends with `controller_invocation_completed`
followed by `task_launch_completed`.

Inspect-only output
`.artifacts/casc-benchmark/casc2025-next-inspect-260808T0124Z.json`, SHA-256
`6917d64465752320883526a32c26cd84c007ade379ac31cc492344549beeaeb9`,
selects outer release `j13` but next release `casc2025` at exactly 0/5,802
results with status `resume_candidate`.  Conclusion: J13 is complete,
quiescent, independently reproducible, and safely torn down.  This does not
complete the campaign: all 5,802 CASC-2025 records remain outstanding, so the
matrix Bead stays in progress and the next guarded slice must begin CASC-2025
from this exact checkpoint.

## Verified first CASC-2025 checkpoint at 636 results

Question: did the first guarded CASC-2025 slice start from the exact completed
J13 boundary, preserve its single service identity, capture a reproducible
successor, and delete only its managed provider resources?  The answer is yes.
Plan
`.artifacts/casc-benchmark/casc2025-0-next-resume-plan-260808T0132Z.json`,
SHA-256
`a66cfeb500fa6fbafd75e817187f483d1ffef1d5d21ca2ac9b04263ec78a46d6`,
selected CASC-2025 at 0/5,802 with a 14,400-second batch wall and 300-second
service margin.  It launched runner `260808-013933-62bb`, label
`e-rust-codex-260808-013933-62bb`, Linode `102462408`, firewall `112024180`,
and IPv4 `172.236.122.207` on `g7-highmem-8` in `us-ord`.  The uploaded source
snapshot SHA-256 was
`736ed5cd32b750a228eb5bbb352ca61b256423871eefce5cd691e0fb7b7059ff`;
package-maintenance quiescence was bound to SHA-256
`c7abc66c60c4bf62dbbf8896beae59ef4e3c2b939a031a7bc791d8b0425952d5`.
All corpus, exact terminal-J13 checkpoint, frozen Umlaut, pinned Vampire,
contract, report, inventory, maintenance, and cgroup-v2 preflights passed.

Service `casc2025-v2-resume-260808-013933-62bb.service` started with MainPID
`4005`, invocation `cb5dba0d173f4bc6a5862451fa4eb72c`, and no restart.
The identity remained fixed through termination.  Its terminal journal has one
boot, `b597582971424bb2981f3e66bb5c87b5`, one invocation, and 641 records.
Sequence `17760` is the unique contract record for
`e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa`
with 636 new and zero resumed results; sequence `17761` is the unique systemd
success record.  Final properties are `Result=success`, `ExecMainStatus=0`,
`NRestarts=0`, and inactive/dead.  Capture removed zero incomplete artifact
pairs and regenerated reports at exactly 636/5,802 CASC-2025, 2,700/2,700
CASC-J13, and 3,336/8,502 combined results.

The ignored successor is
`.artifacts/casc-benchmark/casc2025-checkpoint-260808-013933-62bb.tar.gz`:
44,797,930 bytes with SHA-256
`4d559f466cf2b5385c53155219efaad3801c977c9db4a4d66939ec7d179de493`.
Its 13 regular outer members bind 12 hashes and a unique sorted 636-path
result inventory, SHA-256
`63b78940abe18157a48e13aad2e575396eb7288c68f80b889eb3c3e243649427`.
The nested archive is 61,161,452 bytes with 10,104 regular members and SHA-256
`7966db1ebdffcb96abb76bb37b28e595bff37c0082649efbffc28b8599b6ae59`.
Captured process and service-property evidence hash respectively to
`4d6489e70191cd60c4d4f1cd291a1042cf599aeefe82247c71f8f57d8fb57e76`
and
`f9ac0a431fbe9050b6848bd0c453e6af28d4f314566323eb9053b6344c761122`.

Strict independent validation reproduced contract-file SHA-256
`f895aa07141b091060f3ee46d28f91abd6f484f3ad690630af08a7dbe34284c5`,
manifest SHA-256
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`,
318 Umlaut plus 318 Vampire records, three session records, and CASC-2025
summary SHA-256
`fc2c0fecc63d778d31b2f5c0ac01f095ffae3e5f554d18730044b7b035017dd3`.
It also reproduced the embedded combined boundary and combined summary SHA-256
`240ef6c970e3e3606626301ba93e1c701b062fa56b9f128a3ac48a4ed8e21059`.
Controller and independent validation sidecars are byte-identical, 2,330
bytes, with SHA-256
`4ac5596290aac114cdeae06ca638d1b32d88f9b68ef78840866e44e53a37cb14`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/casc2025-checkpoint-260808-013933-62bb.tar.gz `
  --archive-sha256 4d559f466cf2b5385c53155219efaad3801c977c9db4a4d66939ec7d179de493 `
  --manifest benchmarks/casc_2025_manifest.jsonl `
  --run-name casc30-2025-089e06c8-v2 `
  --contract-id e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --expected-results 636 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/casc2025-checkpoint-260808-013933-62bb.independent-validation.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\casc2025-checkpoint-260808-013933-62bb.tar.gz `
  -ArchiveSha256 4d559f466cf2b5385c53155219efaad3801c977c9db4a4d66939ec7d179de493 `
  -RunName casc30-2025-089e06c8-v2 `
  -ExpectedResults 636 `
  -ContractId e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  -ExpectedNewResults 636 `
  -ExpectedResumedResults 0 `
  -ExpectedContractSequence 17760 `
  -ExpectedSuccessSequence 17761 `
  -ExpectedBootId b597582971424bb2981f3e66bb5c87b5 `
  -ExpectedInvocationId cb5dba0d173f4bc6a5862451fa4eb72c `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-casc2025-260808-0600 `
  -Output .artifacts\casc-benchmark\casc2025-checkpoint-260808-013933-62bb.raw-audit.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/casc2025-checkpoint-260808-013933-62bb.tar.gz `
  --checkpoint-sha256 4d559f466cf2b5385c53155219efaad3801c977c9db4a4d66939ec7d179de493 `
  --inspect-only `
  --output .artifacts/casc-benchmark/casc2025-next-inspect-260808T0557Z.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The reusable raw auditor found 636 coordinates, 1,272 referenced streams,
zero missing/mismatched/orphan streams, three session records, 641 journal
records, no standalone batch/Umlaut/Vampire process, and empty cgroup/unit
residue.  Its ignored JSON output has SHA-256
`ee47da851e75d97b8caea0b850f085f004cf041105c7261d0a2fbf0049b4bd48`.
One result reports `orphan_cleanup_required=true`; this records successful
per-run cleanup rather than surviving residue.  As a falsification check, the
auditor rejected an all-zero archive hash before creating either its fresh
extraction root or output file.  Exact expected boot, invocation, contract,
result, and terminal-sequence values prevent accepting identities merely
copied from the archive.

The controller verified the archive before deleting exact Linode `102462408`
and firewall `112024180`, then recorded `managed_resources_deleted`.  Fresh
runner status is `active: null` with no parked runner; provider check reports
zero restricted-reaper resources; dry GC finds no stale managed resource.
The self-disabled task `Umlaut-CASC-CASC2025-Resume-20260808T013830Z` has last
result zero, and its launch log ends with `controller_invocation_completed`
followed by `task_launch_completed`.

Inspect-only output
`.artifacts/casc-benchmark/casc2025-next-inspect-260808T0557Z.json`, SHA-256
`6e4407462b3ac1692efe3905ea0ceceba3f5bb6b834ccb20a8792d0686fbd4a2`,
selects outer and next release `casc2025` at exactly 636/5,802 with status
`resume_candidate`.  Conclusion: this is a complete, quiescent, independently
reproducible successor with safe provider teardown.  It accepts only this
slice.  CASC-2025 still lacks 5,166 records; with completed J13 included, the
matrix is 3,336/8,502 and Bead `E_Rust_Port-9jt.2.7` remains in progress.  The
next guarded slice must resume CASC-2025 from this exact checkpoint.

## Verified second CASC-2025 checkpoint at 966 results

Question: did the second guarded CASC-2025 slice restore the exact 636-result
predecessor, preserve a single service identity through another complete
session, capture a reproducible successor, and delete only its managed provider
resources?  The answer is yes.  Plan
`.artifacts/casc-benchmark/casc2025-636-next-resume-plan-260808T0607Z.json`,
SHA-256
`238be7a86682f639b3b122433ac3c7e36ab681df70c3a1e02dae862cc4bea91c`,
selected CASC-2025 at 636/5,802 with a 14,400-second batch wall and 300-second
service margin.  Trusted allowance at 2026-08-08T06:08:35Z was 126,000
seconds versus 14,700 required and 18,000 billed.  The self-disabling task
`Umlaut-CASC-CASC2025-Resume-20260808T061335Z` launched runner
`260808-061426-5be0`, label `e-rust-codex-260808-061426-5be0`, Linode
`102473090`, firewall `112359542`, and IPv4 `172.236.122.207` on
`g7-highmem-8` in `us-ord`.  The uploaded 4,187-file/8.9 MiB source snapshot
SHA-256 was
`fe2be7e1884fbaceb4f0b24f00c72ca041c9f8b8e97234980c8379a143dbdc69`;
package-maintenance quiescence was bound to SHA-256
`40939d382a510b855bdae84ed93eb52f65e1eb191172b9d536a336bce420819f`.
All corpus, exact 636-result checkpoint, frozen Umlaut, pinned Vampire,
2,901-problem/2,425-axiom extraction, contract, report, inventory, maintenance,
and strict cgroup-v2 preflights passed.

Service `casc2025-v2-resume-260808-061426-5be0.service` started with MainPID
`4009`, invocation `7b3bc6a9c43e447986e1a35e444e3516`, and no restart.
The identity remained fixed through termination.  Its terminal journal has one
boot, `3a1b8c65ff13445885a77de6a698742d`, one invocation, and 335 records.
Sequence `13507` is the unique contract record for
`e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa`
with 330 new and 636 resumed results; sequence `13508` is the unique systemd
success record.  Final properties are `Result=success`, `ExecMainStatus=0`,
`NRestarts=0`, and inactive/dead.  Capture removed zero incomplete artifact
pairs and regenerated reports at exactly 966/5,802 CASC-2025, 2,700/2,700
CASC-J13, and 3,666/8,502 combined results.

The ignored successor is
`.artifacts/casc-benchmark/casc2025-checkpoint-260808-061426-5be0.tar.gz`:
52,033,969 bytes with SHA-256
`04f2adfa72db2ec7b19bbd6c88ebb80d32211372581eeb8479cc6e0faccad709`.
Its 13 regular outer members bind 12 hashes and a unique sorted 966-path
result inventory, SHA-256
`a3a880555c37c123ab8039fa1de4deb0da49b1bda32058fbc71c2dd78c3d6404`.
The nested archive is 68,475,400 bytes with 11,096 regular members and SHA-256
`d26a1a1c911fd3692f531b6c2320f1e3ef490e06db37b8241a052cc093138460`.
Captured process and service-property evidence hash respectively to
`02acdb02ad8af1f74f3ac15656fd1075f4111513e007375c05f34b7ab95abbdd`
and
`a33b68dfb0dee59b72879449324704a6ebd5af6d9fadb10cdc7249c21207cba8`.

Strict independent validation reproduced contract-file SHA-256
`f895aa07141b091060f3ee46d28f91abd6f484f3ad690630af08a7dbe34284c5`,
manifest SHA-256
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`,
483 Umlaut plus 483 Vampire records, five session records, and CASC-2025
summary SHA-256
`d3b1411e0f199df885e9a41be660d3184770bd8790e4a84021f129e1df573ee2`.
It also reproduced the embedded combined boundary and combined summary SHA-256
`2c22dcc782c45f8699d26a619efd54023a04e7c9304ea3df25a48eb8d3216afb`.
Controller and independent validation sidecars are byte-identical, 2,330
bytes, with SHA-256
`4e1e1e570216f970a59d60ab3d37e94819dfa7ca98dc6e5d88b6aaa2d0721927`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/casc2025-checkpoint-260808-061426-5be0.tar.gz `
  --archive-sha256 04f2adfa72db2ec7b19bbd6c88ebb80d32211372581eeb8479cc6e0faccad709 `
  --manifest benchmarks/casc_2025_manifest.jsonl `
  --run-name casc30-2025-089e06c8-v2 `
  --contract-id e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --expected-results 966 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/casc2025-checkpoint-260808-061426-5be0.independent-validation.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\casc2025-checkpoint-260808-061426-5be0.tar.gz `
  -ArchiveSha256 04f2adfa72db2ec7b19bbd6c88ebb80d32211372581eeb8479cc6e0faccad709 `
  -RunName casc30-2025-089e06c8-v2 `
  -ExpectedResults 966 `
  -ContractId e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  -ExpectedNewResults 330 `
  -ExpectedResumedResults 636 `
  -ExpectedContractSequence 13507 `
  -ExpectedSuccessSequence 13508 `
  -ExpectedBootId 3a1b8c65ff13445885a77de6a698742d `
  -ExpectedInvocationId 7b3bc6a9c43e447986e1a35e444e3516 `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-casc2025-260808-1040 `
  -Output .artifacts\casc-benchmark\casc2025-checkpoint-260808-061426-5be0.raw-audit.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/casc2025-checkpoint-260808-061426-5be0.tar.gz `
  --checkpoint-sha256 04f2adfa72db2ec7b19bbd6c88ebb80d32211372581eeb8479cc6e0faccad709 `
  --inspect-only `
  --output .artifacts/casc-benchmark/casc2025-next-inspect-260808T1043Z.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The reusable raw auditor found 966 coordinates, 1,932 referenced streams,
zero missing/mismatched/orphan streams, five session records, 335 journal
records, no standalone batch/Umlaut/Vampire process, and empty cgroup/unit
residue.  Its ignored JSON output is 856 bytes with SHA-256
`bcfa66a618215a6aaedb3341554c220736c24e89e10a7094e8d2154a9866bb15`.
One historical result reports `orphan_cleanup_required=true`; this records
successful per-run cleanup rather than surviving residue.  As a falsification
check, the auditor rejected an all-zero archive hash before creating either
its fresh extraction root or output file.

The controller verified the archive before deleting exact Linode `102473090`
and firewall `112359542`, then recorded `managed_resources_deleted`.  Fresh
runner status is `active: null` with no parked runner; provider check reports
zero restricted-reaper resources; dry GC finds no stale managed resource.  The
self-disabled task has last result zero, and its launch log ends with
`controller_invocation_completed` followed by `task_launch_completed`.

Inspect-only output
`.artifacts/casc-benchmark/casc2025-next-inspect-260808T1043Z.json`, SHA-256
`9687e4e19524c4474d6a3da008e1755cc527ed464df29bf42fd5a28cd05e60a1`,
selects outer and next release `casc2025` at exactly 966/5,802 with status
`resume_candidate`.  Conclusion: this is a complete, quiescent, independently
reproducible successor with safe provider teardown.  It accepts only this
slice.  CASC-2025 still lacks 4,836 records; with completed J13 included, the
matrix is 3,666/8,502 and Bead `E_Rust_Port-9jt.2.7` remains in progress.  The
next guarded slice must resume CASC-2025 from this exact checkpoint.

## Verified third CASC-2025 checkpoint at 1,307 results

Question: did the third guarded CASC-2025 slice restore the exact 966-result
predecessor, retain a single service identity, capture every completed result
at an odd solver boundary, and safely remove its managed provider resources?
The answer is yes.  Plan
`.artifacts/casc-benchmark/casc2025-966-next-resume-plan-260808T1045Z.json`,
SHA-256
`166969e0d20a393db07cf3ffc8fb8824be34fabb99de08d12741ac4e3adb76a4`,
selected CASC-2025 at 966/5,802 with a 14,400-second batch wall and 300-second
service margin.  Trusted allowance at 2026-08-08T10:45:51Z was 108,000
seconds versus 14,700 required and 18,000 billed.  The audited self-disabling
task `Umlaut-CASC-CASC2025-Resume-20260808T105051Z` launched runner
`260808-105141-c593`, label `e-rust-codex-260808-105141-c593`, Linode
`102483090`, firewall `112737032`, and IPv4 `172.236.122.207` on
`g7-highmem-8` in `us-ord`.  The uploaded 4,187-file/8.9 MiB source snapshot
SHA-256 was
`d4af6e93327f8ea82e3a0b2d89aa4581b69157f98fa15c78ee24d00ea62c2d29`;
package-maintenance quiescence was bound to SHA-256
`33baebda1b54ad604d05d96afaadf064add9cff274e7dab54c59013bcbc93cd5`.
All corpus, exact 966-result checkpoint, frozen Umlaut, pinned Vampire,
2,901-problem/2,425-axiom extraction, contract, report, inventory, maintenance,
and strict cgroup-v2 preflights passed.

Service `casc2025-v2-resume-260808-105141-c593.service` started with MainPID
`4011`, invocation `013fb8bbadf742c3bc8ca1842bb653d8`, and no restart.
The identity remained fixed through termination.  Its terminal journal has one
boot, `b45160633d774e1db86f9d1eceb92768`, one invocation, and 346 records.
Sequence `19806` is the unique contract record for
`e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa`
with 341 new and 966 resumed results; sequence `19807` is the unique systemd
success record.  Final properties are `Result=success`, `ExecMainStatus=0`,
`NRestarts=0`, and inactive/dead.  Capture removed zero incomplete artifact
pairs and regenerated reports at exactly 1,307/5,802 CASC-2025, 2,700/2,700
CASC-J13, and 4,007/8,502 combined results.

The ignored successor is
`.artifacts/casc-benchmark/casc2025-checkpoint-260808-105141-c593.tar.gz`:
61,129,422 bytes with SHA-256
`4b14aa72c2955297f6860b91cc354808275cf0bc95ffea4a3737f19899dc66cf`.
Its 13 regular outer members bind 12 hashes and a unique sorted 1,307-path
result inventory, SHA-256
`c175b46d96df86492cf975627557762116b9fae18e470cc20d696e061194e57e`.
The nested archive is 77,938,384 bytes with 12,121 regular members and SHA-256
`e76952053425a2cea53f2c8d9dadef2761250ead68747045801c4107af48a586`.
Captured process and service-property evidence hash respectively to
`d8859f144972cde414510e08e02288d17a3947db07d860aae95800b8d38754ff`
and
`5a0ab04c556360b20aa803f359292063d3f6f95a4e90a6fe27fb2c38ae57baee`.

Strict independent validation reproduced contract-file SHA-256
`f895aa07141b091060f3ee46d28f91abd6f484f3ad690630af08a7dbe34284c5`,
manifest SHA-256
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`,
654 Umlaut plus 653 Vampire records, seven session records, and CASC-2025
summary SHA-256
`c34f796dff25757bc29597698142014ac644d576368694df5ebec6d99acbb162`.
It also reproduced the embedded combined boundary and combined summary SHA-256
`71e610a2cc2ef5c578aa0f78140c742e5ec7b3d4a019ee1ae0f0f37c081db69e`.
Controller and independent validation sidecars are byte-identical, 2,333
bytes, with SHA-256
`758daa1dbad5f3a8e65f30b1967b392391419c8da1ea5753376ccd7bba928298`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/casc2025-checkpoint-260808-105141-c593.tar.gz `
  --archive-sha256 4b14aa72c2955297f6860b91cc354808275cf0bc95ffea4a3737f19899dc66cf `
  --manifest benchmarks/casc_2025_manifest.jsonl `
  --run-name casc30-2025-089e06c8-v2 `
  --contract-id e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --expected-results 1307 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/casc2025-checkpoint-260808-105141-c593.independent-validation.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\casc2025-checkpoint-260808-105141-c593.tar.gz `
  -ArchiveSha256 4b14aa72c2955297f6860b91cc354808275cf0bc95ffea4a3737f19899dc66cf `
  -RunName casc30-2025-089e06c8-v2 `
  -ExpectedResults 1307 `
  -ContractId e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  -ExpectedNewResults 341 `
  -ExpectedResumedResults 966 `
  -ExpectedContractSequence 19806 `
  -ExpectedSuccessSequence 19807 `
  -ExpectedBootId b45160633d774e1db86f9d1eceb92768 `
  -ExpectedInvocationId 013fb8bbadf742c3bc8ca1842bb653d8 `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-casc2025-260808-1513 `
  -Output .artifacts\casc-benchmark\casc2025-checkpoint-260808-105141-c593.raw-audit.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/casc2025-checkpoint-260808-105141-c593.tar.gz `
  --checkpoint-sha256 4b14aa72c2955297f6860b91cc354808275cf0bc95ffea4a3737f19899dc66cf `
  --inspect-only `
  --output .artifacts/casc-benchmark/casc2025-next-inspect-260808T1515Z.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The reusable raw auditor found 1,307 coordinates, 2,614 referenced streams,
zero missing/mismatched/orphan streams, seven session records, 346 journal
records, no standalone batch/Umlaut/Vampire process, and empty cgroup/unit
residue.  Its ignored JSON output is 858 bytes with SHA-256
`c3a7fff98abecd2c8d18d9de64b1f7fe33dc15cc2faac702b79a3efba57c1f6d`.
Two historical results report `orphan_cleanup_required=true`; these record
successful per-run cleanup rather than surviving residue.  As a falsification
check, the auditor rejected an all-zero archive hash before creating either
its fresh extraction root or output file.

The controller verified the archive before deleting exact Linode `102483090`
and firewall `112737032`, then recorded `managed_resources_deleted`.  Fresh
runner status is `active: null` with no parked runner; provider check reports
zero restricted-reaper resources; dry GC finds no stale managed resource.  The
self-disabled task has last result zero, and its launch log ends with
`controller_invocation_completed` followed by `task_launch_completed`.

Inspect-only output
`.artifacts/casc-benchmark/casc2025-next-inspect-260808T1515Z.json`, SHA-256
`fd96f2e076d835dfb4cb7838ba271a13d610847b095a8e03215f74f936cdbcdf`,
selects outer and next release `casc2025` at exactly 1,307/5,802 with status
`resume_candidate`.  Conclusion: this is a complete, quiescent, independently
reproducible successor with safe provider teardown.  It accepts only this
slice.  CASC-2025 still lacks 4,495 records; with completed J13 included, the
matrix is 4,007/8,502 and Bead `E_Rust_Port-9jt.2.7` remains in progress.  The
next guarded slice must resume CASC-2025 from this exact checkpoint.

## Verified fourth CASC-2025 checkpoint at 1,555 results

Question: did the fourth guarded CASC-2025 slice resume the exact 1,307-result
checkpoint, preserve a single service identity, capture its successor, and
delete only its managed resources?  The answer is yes.  Plan
`.artifacts/casc-benchmark/casc2025-1307-next-resume-plan-260808T1517Z.json`,
SHA-256
`dd0fe9b0d7f7f95adf6d938537eaafcab725c9c14d20feaaf894cd8e59a83cae`,
selected CASC-2025 at 1,307/5,802 with a 14,400-second batch wall and
300-second service margin.  Trusted allowance at 2026-08-08T15:17:58Z was
90,000 seconds versus 14,700 required and 18,000 billed.  The audited
self-disabling task `Umlaut-CASC-CASC2025-Resume-20260808T152258Z` launched
runner `260808-152349-f49c`, label `e-rust-codex-260808-152349-f49c`, Linode
`102490893`, firewall `113066735`, and IPv4 `172.236.122.207` on
`g7-highmem-8` in `us-ord`.  The uploaded 4,187-file/8.9 MiB source snapshot
SHA-256 was
`8d31e46050042ad703aa9bc8b2147d1b06160cef6f3eeb0067b72aadd9fc682f`;
package-maintenance quiescence was bound to SHA-256
`6b531cb5fdadaf838e14c935393762419a99a89d27a7b3b342a65a080295dd4e`.
All corpus, exact predecessor checkpoint, frozen binaries, safe extraction,
contract, report, inventory, maintenance, and strict cgroup-v2 preflights
passed.

Service `casc2025-v2-resume-260808-152349-f49c.service` started with MainPID
`3992`, invocation `70792f71f3344cb0aafef2e61f08abf4`, and no restart.
The identity remained fixed through termination.  Its terminal journal has one
boot, `06b04aba31d744348bfe22ee771341d6`, one invocation, and 253 records.
Sequence `16580` is the unique contract record with 248 new and 1,307 resumed
results; sequence `16581` is the unique systemd success record.  Final
properties are `Result=success`, `ExecMainStatus=0`, `NRestarts=0`, and
inactive/dead.  Capture removed zero incomplete artifact pairs and regenerated
reports at exactly 1,555/5,802 CASC-2025, 2,700/2,700 CASC-J13, and
4,255/8,502 combined results.

The ignored successor is
`.artifacts/casc-benchmark/casc2025-checkpoint-260808-152349-f49c.tar.gz`:
63,754,394 bytes with SHA-256
`849d56625e95a84cfe679cb84d5693ed3956c464b1b66af31790d732f1d48253`.
Its 13 regular outer members bind 12 hashes and a unique sorted 1,555-path
result inventory, SHA-256
`e2fa8fd3246665e2e522e87708c106734aa020e60329e2f4d14eb30da280ce24`.
The nested archive is 81,534,365 bytes with 12,867 regular members and SHA-256
`ab332087c7c9453f44d4952261ea14ac5cd8a5b8e493b64f637790d3e19b25f1`.
Captured process and service-property evidence hash respectively to
`0406f1e582a62a9565a170f347628708ef438b5cf780f3cee72c19a72e11e3f6`
and
`a7f49911852672ec1bd037289629810aa7f3f44f64dfa800589cefb0455cb45f`.

Strict independent validation reproduced 778 Umlaut plus 777 Vampire records,
nine session records, CASC-2025 summary SHA-256
`445b107580c6934320d42eecb273925b637c696b4417d893dcc0efec99d4d12c`,
and embedded combined summary SHA-256
`57a067f8cf1ceae3af92358f15607aa66ce233bca42076a55cb8e20fb7dab0f8`.
Controller and independent validation sidecars are byte-identical, 2,333
bytes, with SHA-256
`a1ebdf916d91af3309eaadbfa9acb003a019f563fa684cd2a8ad8960084fdf26`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/casc2025-checkpoint-260808-152349-f49c.tar.gz `
  --archive-sha256 849d56625e95a84cfe679cb84d5693ed3956c464b1b66af31790d732f1d48253 `
  --manifest benchmarks/casc_2025_manifest.jsonl `
  --run-name casc30-2025-089e06c8-v2 `
  --contract-id e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --expected-results 1555 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/casc2025-checkpoint-260808-152349-f49c.independent-validation.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\casc2025-checkpoint-260808-152349-f49c.tar.gz `
  -ArchiveSha256 849d56625e95a84cfe679cb84d5693ed3956c464b1b66af31790d732f1d48253 `
  -RunName casc30-2025-089e06c8-v2 `
  -ExpectedResults 1555 `
  -ContractId e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  -ExpectedNewResults 248 `
  -ExpectedResumedResults 1307 `
  -ExpectedContractSequence 16580 `
  -ExpectedSuccessSequence 16581 `
  -ExpectedBootId 06b04aba31d744348bfe22ee771341d6 `
  -ExpectedInvocationId 70792f71f3344cb0aafef2e61f08abf4 `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-casc2025-260808-1947 `
  -Output .artifacts\casc-benchmark\casc2025-checkpoint-260808-152349-f49c.raw-audit.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/casc2025-checkpoint-260808-152349-f49c.tar.gz `
  --checkpoint-sha256 849d56625e95a84cfe679cb84d5693ed3956c464b1b66af31790d732f1d48253 `
  --inspect-only `
  --output .artifacts/casc-benchmark/casc2025-next-inspect-260808T1949Z.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The reusable raw auditor found 1,555 coordinates, 3,110 referenced streams,
zero missing/mismatched/orphan streams, nine session records, 253 journal
records, no standalone batch/Umlaut/Vampire process, and empty cgroup/unit
residue.  Its ignored JSON output is 858 bytes with SHA-256
`ceaf76955e56893326eb45661dc44d254d8ab4ea68f672ff7fb9fb951d10359f`.
Five historical results report `orphan_cleanup_required=true`; these record
successful per-run cleanup rather than surviving residue.  The all-zero
archive-hash falsification failed before creating an extraction root or output.

The controller verified the archive before deleting exact Linode `102490893`
and firewall `113066735`, then recorded `managed_resources_deleted`.  Fresh
runner status is empty, restricted-reaper validation passes with zero
resources, and dry GC finds nothing stale.  The self-disabled task has last
result zero and its launch log contains both completion markers.

Inspect-only output
`.artifacts/casc-benchmark/casc2025-next-inspect-260808T1949Z.json`, SHA-256
`6617ad7d0dec1c2c468266aa0f0336f3e22e3d330a32dcf7175cc3635e993f28`,
selects CASC-2025 at exactly 1,555/5,802 with status `resume_candidate`.
Conclusion: this is a complete, quiescent, independently reproducible
successor with safe provider teardown.  CASC-2025 still lacks 4,247 records;
with completed J13 included, the matrix is 4,255/8,502 and Bead
`E_Rust_Port-9jt.2.7` remains in progress.  The next guarded slice must resume
CASC-2025 from this exact checkpoint.

## Verified fifth CASC-2025 checkpoint at 1,794 results

The authorized successor plan
`.artifacts/casc-benchmark/casc2025-1555-next-resume-plan-260808T1954Z.json`,
SHA-256
`dac99f89cd0c6c9741878262573d01908266a33b035bbe8194b3c6f41587c88d`,
strictly revalidated the 1,555-result predecessor and observed 72,000 allowance
seconds versus 14,700 required and 18,000 billed.  The audited, self-disabling
task `Umlaut-CASC-CASC2025-Resume-20260808T200006Z` launched controller log
`.artifacts/casc-benchmark/casc2025-resume-controller-20260808T200054Z-18608.log`.
It created runner `260808-200100-cf23`, Linode `102498606`, firewall
`113393952`, and IP `172.236.122.207`.  The source snapshot contained 4,187
files (8.9 MiB), SHA-256
`0dae5a46fc5033a2603a33b7596cd83f0efe011db150ba135fb5e9831241416a`;
maintenance-quiescence evidence hashes to
`290ba97137f3b1c132c5236e60331f58175056f406f0f2e145b217b00972b3ed`.
All payload hashes, safe corpus extraction, checkpoint members, exact restored
inventory and summary, historical contract compatibility, and strict cgroup-v2
preflight passed before solver launch.

Service `casc2025-v2-resume-260808-200100-cf23.service` started with MainPID
`3999`, invocation `a41f3ded1a594410a6c169147154a361`, and no restart.  Its
identity remained fixed through termination.  The terminal journal has boot
`2f76e02c735e43019e1a7196b3a4cdd1`, one invocation, and 244 records.
Sequence `13461` is the unique contract record with 239 new and 1,555 resumed
results; sequence `13462` is the unique systemd success record.  Final
properties are `Result=success`, `ExecMainStatus=0`, `NRestarts=0`, and
inactive/dead.  Capture removed zero incomplete artifact pairs and regenerated
reports at exactly 1,794/5,802 CASC-2025, 2,700/2,700 CASC-J13, and
4,494/8,502 combined results.

The ignored successor is
`.artifacts/casc-benchmark/casc2025-checkpoint-260808-200100-cf23.tar.gz`:
67,525,759 bytes with SHA-256
`82ebce0f08fb290b4c76b96b99d2d18fe8ac06fe1fcab37046b200510c078a7d`.
Its 13 regular outer members bind 12 hashes and a unique sorted 1,794-path
result inventory, SHA-256
`42afb260f7915334b269a41009dd967ea27f987de92577725767140263f91ae1`.
The nested archive is 85,475,267 bytes with 13,586 regular members and SHA-256
`0db770ff8ed8a013a97922584e95518a879a9c052724ec6f321725e5bc6ac801`.
Captured process and service-property evidence hash respectively to
`b52c1814505bb33b699d4b2c0f7ba7c9c7e078968ceb45f442b7fec987abc8e9`
and
`8ccd89b5fb105e43276eca9d2cc988897134e83f66336d26a73c2578ce2546f1`.

Strict independent validation reproduced 897 Umlaut plus 897 Vampire records,
11 session records, CASC-2025 summary SHA-256
`0bf1b9aa8ae9f6228da6190db65bd4017667fe733fe333efb71e39dae9b9df80`,
and embedded combined summary SHA-256
`05bfedd33b48a603ccd67322dbd95b5942d0157358c30757552581c156ac072a`.
Controller and independent validation sidecars are byte-identical, 2,334
bytes, with SHA-256
`1a11018fb8a0f9274b170c043ea5674c7565b3e4361bfba641b2aef1c34f174d`.

Reproduction from the repository root is:

```powershell
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/validate_casc_checkpoint.py `
  --archive .artifacts/casc-benchmark/casc2025-checkpoint-260808-200100-cf23.tar.gz `
  --archive-sha256 82ebce0f08fb290b4c76b96b99d2d18fe8ac06fe1fcab37046b200510c078a7d `
  --manifest benchmarks/casc_2025_manifest.jsonl `
  --run-name casc30-2025-089e06c8-v2 `
  --contract-id e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --expected-results 1794 `
  --combined-run CASC-2025 benchmarks/casc_2025_manifest.jsonl `
    casc30-2025-089e06c8-v2 `
    e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  --combined-run CASC-J13 benchmarks/casc_2026_manifest.jsonl `
    casc-j13-2026-089e06c8-v2 `
    9f29cac72abe79a5a0b31f5135412243f95ec344b5152eadc3d372ac49e8c676 `
  --output .artifacts/casc-benchmark/casc2025-checkpoint-260808-200100-cf23.independent-validation.json
.\experiments\2026-07-28-004-casc-benchmark-matrix\audit_casc_checkpoint.ps1 `
  -Archive .artifacts\casc-benchmark\casc2025-checkpoint-260808-200100-cf23.tar.gz `
  -ArchiveSha256 82ebce0f08fb290b4c76b96b99d2d18fe8ac06fe1fcab37046b200510c078a7d `
  -RunName casc30-2025-089e06c8-v2 `
  -ExpectedResults 1794 `
  -ContractId e71fc642a15db4528fb915724493b7571798fe40848a4fe0085e62723918d1aa `
  -ExpectedNewResults 239 `
  -ExpectedResumedResults 1555 `
  -ExpectedContractSequence 13461 `
  -ExpectedSuccessSequence 13462 `
  -ExpectedBootId 2f76e02c735e43019e1a7196b3a4cdd1 `
  -ExpectedInvocationId a41f3ded1a594410a6c169147154a361 `
  -ExtractionRoot C:\tmp\umlaut-casc-audit-casc2025-260809-0035 `
  -Output .artifacts/casc-benchmark/casc2025-checkpoint-260808-200100-cf23.raw-audit.json
.\.venv\Scripts\python.exe -u `
  experiments/2026-07-28-004-casc-benchmark-matrix/plan_next_casc_resume.py `
  --checkpoint .artifacts/casc-benchmark/casc2025-checkpoint-260808-200100-cf23.tar.gz `
  --checkpoint-sha256 82ebce0f08fb290b4c76b96b99d2d18fe8ac06fe1fcab37046b200510c078a7d `
  --inspect-only `
  --output .artifacts/casc-benchmark/casc2025-next-inspect-260809T0038Z.json
.\linode-runner.ps1 status
.\linode-runner.ps1 check
.\linode-runner.ps1 gc --older-than-hours 1
```

The reusable raw auditor found 1,794 coordinates, 3,588 referenced streams,
zero missing/mismatched/orphan streams, 11 session records, 244 journal
records, no standalone batch/Umlaut/Vampire process, and empty cgroup/unit
residue.  Its ignored JSON output is 860 bytes with SHA-256
`616b2ec32a708583a327ecfcadc7a265a823c4e7ff57a5b9a3ec17f50289bcc4`.
Eleven historical results report `orphan_cleanup_required=true`; these record
successful per-run cleanup rather than surviving residue.  The all-zero
archive-hash falsification failed before creating an extraction root or output.

The controller verified the archive before deleting exact Linode `102498606`
and firewall `113393952`, then recorded `managed_resources_deleted`.  Fresh
runner status is empty, restricted-reaper validation passes with zero
resources, and dry GC finds nothing stale.  The self-disabled task has last
result zero and its launch log contains both completion markers.

Inspect-only output
`.artifacts/casc-benchmark/casc2025-next-inspect-260809T0038Z.json`, SHA-256
`1cc0334207f76113c68a563ffbdad999326852fcb3f531a67fd08aafe96c8a93`,
selects CASC-2025 at exactly 1,794/5,802 with status `resume_candidate`.
Conclusion: this is a complete, quiescent, independently reproducible
successor with safe provider teardown.  CASC-2025 still lacks 4,008 records;
with completed J13 included, the matrix is 4,494/8,502 and Bead
`E_Rust_Port-9jt.2.7` remains in progress.  The next guarded slice must resume
CASC-2025 from this exact checkpoint.

## Remaining acceptance boundary

The guarded campaign has now validated program construction, separate ignored
inputs, binary and corpus hashes, eight-core/128 GiB execution, cgroup
accounting, SZS extraction, atomic results, repeated exact resume, report
generation, artifact transfer, and cleanup across all 1,350 J13 problems and
both solvers, plus 897 Umlaut and 897 Vampire CASC-2025 executions.  Gate
`E_Rust_Port-9jt.2.7` still preserves the broader full-run acceptance work:
2,004 Umlaut and 2,004 Vampire CASC-2025 executions remain.  The J13 portion is
complete at 2,700 records; 1,794/5,802 CASC-2025 records are complete, leaving
4,008 CASC-2025 executions and the final complete combined reports outstanding.
