# Cooperative multicore search findings

## Conclusion

Do not change Umlaut's production scheduler or automatic schedules from this
experiment. The preregistered final verdict is `uncertain`, with no selected
arm.

The strongest candidate, `share_64`, produced a real signal: it reproducibly
solved validation problem `LCL365+1` while the equal independent, unequal
independent, and restart-only controls all gave up. Both winning proofs passed
the repository validation gate and ProofCheck 1.0 against the untouched
original problem and the watchlist wrapper. The signal did not repeat on the
separate test families. Test had one reproducible solve, `NUN060+1`, and every
arm solved it.

The result is not an adoption result because the frozen rule required either a
reproducible test-only solve or at least four common solved coordinates against
the restart control in both validation and test. Validation had zero such
coordinates and test had two. The result is not a `stop` result because
validation and test disagree: `share_64` has a reproducible validation-only
solve and no reproducible held-out loss.

Production source and schedules are unchanged.

## Scope and frozen design

This experiment evaluates Bead `E_Rust_Port-9jt.3.7`. It reuses the
candidate-blind, family-held-out 32-problem corpus from experiment 018:

```text
experiments/2026-07-29-018-tsm-learning-baseline/corpus.jsonl
SHA-256 28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1
```

The split has 16 training, 8 validation, and 8 test problems. FNE, FEQ, EPU,
and UEQ each contribute two problems to each held-out split, and source
families do not cross split boundaries.

Every coordinate reserved four pinned cores and at most 16 configured
aggregate prover CPU-seconds. Four deterministic workers used distinct
heuristics and explicit `RandomWeight` seeds. The six arms were:

1. four uninterrupted four-second workers;
2. train-selected unequal slices of 7, 3, 2, and 4 seconds;
3. a restart-only 1 + 1 + 2 second control;
4. the same restart schedule with at most 4 peer watchlist clauses;
5. the same schedule with at most 16 peer watchlist clauses; and
6. the same schedule with at most 64 peer watchlist clauses.

The unequal mapping was frozen from training as worker order `[0, 3, 1, 2]`
and budgets `[7, 3, 2, 4]`, under selection id
`bbbc61124cd236fa1f5c3945e9f3d7bfc8cbf4390e3b616e16c242ebc24231e3`.

Shared clauses came only from another worker on the current problem. They were
short, deterministically ranked processed clauses written with the TPTP
`watchlist` role and consumed through `--static-watchlist`; they were heuristic
targets, not logical premises. Each continuation restarted from the untouched
problem plus a static watchlist wrapper. No live `ProofState` was shared.

## Results

Training ran once per arm/problem. Validation and test ran twice, and a held-out
solve counts only when both repetitions agree.

| Arm | Train solves | Validation reproducible solves | Test reproducible solves |
| --- | ---: | ---: | ---: |
| independent equal | 5 | 1 | 1 |
| independent unequal | 4 | 1 | 1 |
| restart control | 4 | 0 | 1 |
| share 4 | 4 | 1 | 1 |
| share 16 | 4 | 1 | 1 |
| share 64 | 4 | 2 | 1 |

Training's equal portfolio alone solved `CSR052+3`. Every other arm solved the
same four training problems. That loss is selection evidence, not a held-out
failure, but it shows that the checkpoint/restart schedule can discard useful
uninterrupted progress.

On validation:

- equal, unequal, `share_4`, and `share_16` reproducibly solved only
  `PUZ037-2`;
- restart control solved no problem;
- `share_64` reproducibly solved both `PUZ037-2` and the control-unique
  `LCL365+1`; and
- equal slicing solved `LCL026-10` in only one repetition, so it was correctly
  excluded from reproducible solve and loss sets.

On test, all six arms reproducibly solved only `NUN060+1`. No arm had a
one-repeat solve, unique solve, or reproducible loss.

The bounded exchange channel was active rather than a no-op:

| Arm | Train clauses | Validation clauses | Test clauses |
| --- | ---: | ---: | ---: |
| share 4 | 384 | 500 | 448 |
| share 16 | 1,464 | 1,940 | 1,756 |
| share 64 | 5,212 | 7,332 | 6,450 |

There were no malformed, self-only, or premise-promoting exchange failures.

## Efficiency evidence

Validation provides no paired efficiency comparison against restart because
restart solved no validation problem. Test provides only the two
`NUN060+1` repetition coordinates, below the frozen minimum of four:

| Arm versus restart | Common test coordinates | Median CPU ratio | Median wall ratio | Median peak-RSS ratio |
| --- | ---: | ---: | ---: | ---: |
| share 4 | 2 | 0.995 | 1.004 | 1.160 |
| share 16 | 2 | 0.548 | 0.661 | 0.916 |
| share 64 | 2 | 0.846 | 0.886 | 1.092 |

The favorable `share_16` and `share_64` time ratios are therefore exploratory,
not advancement evidence. `share_64` also misses the 1.05 RSS threshold on
those two coordinates.

Unequal slicing adds no held-out solve over equal slicing. It has only two
common test coordinates and therefore does not advance under the same rule.

The separate preprocessing audit reproduced canonical-CNF hashes. Measured
four-conversion versus one-conversion CPU totals were 12.39 versus 2.32 seconds
in training, 0.12 versus 0.01 in validation, and 0.02 versus a rounded 0.00 in
test. Shared preprocessing does not advance because the held-out redundant
cost is below the frozen threshold and no original-problem proof reconstruction
path was demonstrated for a shared-CNF search arm.

## Correctness and falsification

The accepted matrix contains 288 coordinates:

- 96 training coordinates under contract
  `a813a5ce5f9980d5afa0e4267212e87d4f1b421d93a83ff3b1c78743879fec90`;
- 96 validation coordinates under contract
  `e1dabf11e7bef82255fc47d28d277015e309a12f5792c734aa3d5565bfdc881e`;
  and
- 96 test coordinates under contract
  `5d1483df0ae9cb5b592ba69c26253a6985ccaa1006bd1757c35c1cfaed28a3ec`.

Independent validation accepted every coordinate and replayed all 50 recorded
winner proofs: 25 training, 13 validation, and 12 test. Proof-status winners
were rerun alone as annotated TSTP, checked against the untouched original
problem, and checked against the wrapper for cooperative runs. All source,
binary, script, stdout, stderr, telemetry, timing, wrapper, and recorded proof
hashes matched, and every fresh independent replay verified. No worker left a
surviving process or temporary-file residue.

The phase controller hashes each top-level problem and binds the exact corpus
manifest; TPTP include files are established at corpus reconstruction rather
than rehashed in every coordinate. A final independent reconstruction from the
integrity-pinned CASC archive prepared the same 32 problems and 18 include
files. `diff -qr` found the complete reconstructed tree byte-identical to the
measured corpus.

Thirteen focused Python tests passed on Linux. They cover saturated-clause
parsing and rejection, novelty, deterministic peer ranking, static-watchlist
rendering, archive traversal rejection, worker seeds, CNF extraction, and
final-decision behavior.

An exact resume of all 96 test coordinates completed in 0.9 seconds without
rerunning prover work. Reanalysis reproduced byte-identical
`test-analysis.json` with SHA-256
`d448088fa68daa88bbbd82cdd3920e999122a5a4f767ca4f21c8a95bde75f3f1`.

For an independent negative control, one measured telemetry file was backed up
and truncated. The validator rejected it with the expected worker-telemetry
hash mismatch. Restoring the original
`15dd05a980736d62b4b1517857f44ddd6735252f737b12fc50766994f52ff37a`
file hash returned the complete test validation to 96 coordinates, 12 proof
replays, and zero failures. The temporary copy and its derived output were
deleted.

## Diagnostics before held-out execution

No validation or test result was opened before its phase was permitted by the
frozen inputs.

An initial completed training tree, `train-v1`, was rejected before validation
opened. Proof replay had reused the primary worker's telemetry path and
overwritten a measured artifact. The independent validator detected the hash
mismatch. Replay telemetry was isolated, resume hashing was strengthened, and
the full training phase was rerun from an empty `train-v2` root. `train-v1`,
its selection, and its reports are excluded from the accepted evidence.

Earlier smoke diagnostics also changed proof replay from PCL text to annotated
TSTP because the repository validation gate correctly requires TSTP, and
changed saturated-output parsing to ignore ordinary unannotated input clauses
while continuing to reject malformed annotated records. Both fixes preceded
the accepted training run and did not change the preregistered corpus, arms,
budgets, caps, or decision rule.

## Decision

All correctness gates pass. `share_64` loses no reproducible validation or test
solve versus any control and has one reproducible validation-only solve.
However:

- it has no reproducible test-only solve;
- validation has zero common solved coordinates versus restart;
- test has only two common solved coordinates versus restart; and
- validation/test therefore disagree about unique coverage.

The preregistered decision is:

```text
verdict: uncertain
selected_arm: null
reason: insufficient_common_solved_coordinates
```

The process-isolated portfolio remains the production default. The 64-clause
same-problem watchlist design is a credible candidate for a larger,
longer-budget follow-up, but this experiment does not authorize integration.

## Evidence

The measured source revision is
`77a42527467d01f17a6045852f57d3498d93de23`. The release Umlaut SHA-256 is
`c3493604f0d5be15c04a5b2a3f14dfa30e672edea6ae4bab94c5353169d55e65`;
ProofCheck is
`92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`;
and the repository validation gate is
`4c90eea3faa207af374f6c000276f7d1268e64ecbf13a78800b29abf399733d0`.

The final decision report has SHA-256
`08a4175a1e1e0282e773f21e65e67eee37eaf3bcfde1ae8b47e57cbb85935b1e`.
The ignored accepted evidence archive is:

```text
.artifacts/experiments/2026-07-30-009-cooperative-multicore-search/evidence.tar.gz
bytes: 40,149,355
entries: 16,902
SHA-256 cd4e33027cd5756438a53f22443baf1420786da6295cfa99a78b560c3e78df30
```

It contains only the accepted train/validation/test result trees, independent
replay outputs, selection, analyses, validator reports, final decision, and
corpus report. It excludes the TPTP source corpus, ProofCheck distribution,
smoke diagnostics, and invalid `train-v1` artifacts.

This is a 32-problem, family-held-out, four-core, short-budget study. Static
restart-and-watchlist exchange approximates bounded cooperation without
modifying live Rust state; it does not establish the behavior of native
lock-free clause exchange, longer schedules, or CASC hardware.
