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
