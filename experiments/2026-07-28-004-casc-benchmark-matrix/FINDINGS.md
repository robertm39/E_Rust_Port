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
task remains `Ready` for `2026-08-03T05:00:10Z`.

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
