# Preregistration: cooperative multicore search

## Question and hypothesis

This experiment addresses Bead `E_Rust_Port-9jt.3.7`.

Can deterministic, same-problem exchange of a very small set of peer-produced
clauses improve a four-process Umlaut portfolio over independent workers on the
same hardware and configured aggregate CPU budget?

The hypothesis is deliberately narrow. Sharing all clauses is expected to
destroy diversity and increase memory pressure. A small static watchlist of
short clauses found by other workers may instead guide a continuation toward
peer progress without adding logical premises. The result may reject this
hypothesis.

## Frozen source and prior evidence

The measured source revision is
`77a42527467d01f17a6045852f57d3498d93de23`.

The experiment reuses the candidate-blind, family-held-out corpus from
experiment 018:

```text
experiments/2026-07-29-018-tsm-learning-baseline/corpus.jsonl
SHA-256 28b6ac9d59d2871877a7b784b41bc70fe5c09386da6214123791e660819b67c1
```

It has 16 train, 8 validation, and 8 test problems, with two problems from each
of FNE, FEQ, EPU, and UEQ in both held-out splits. Source families are
disjoint between train, validation, and test. Only expected theorem or
unsatisfiable problems are present.

Prior experiment 019 found no value from cross-problem proof-clause
watchlists. This experiment does not repeat that transfer: every shared clause
is produced from the current target by a different worker during the current
coordinate. Prior experiment 020 found that one-second hard stops often did
not emit telemetry. Probes here therefore have deterministic processed-clause
checkpoints and print only processed clause sets.

No validation or test output may change the corpus, worker strategies,
checkpoint counts, sharing caps, budgets, metrics, or decision rule below.

## Hardware and isolation

The primary experiment requires a Linux host exposing exactly four usable
CPUs. One coordinate runs at a time. Its four workers are pinned one-to-one to
the frozen CPU list, use separate process groups and files, and have a
1,536 MiB per-process memory limit. The controller samples aggregate resident
memory across all live worker groups, cancels losing groups after a proof, and
escalates from `SIGTERM` to `SIGKILL` after one second.

The four-core Linode is a controlled proxy, not a CASC or StarExec machine.
Absolute timing is not generalized to those environments.

## Deterministic worker diversity

Every worker uses KBO6 and a three-queue heuristic. The dominant queue differs
between workers:

1. global refined weight with `ConstPrio`;
2. refined weight with `PreferGoals`;
3. refined weight with `PreferNonGoals`;
4. orient-lmax weight with `ConstPrio`.

Each worker also has a `RandomWeight` queue with an explicit, distinct
three-integer seed and a FIFO queue. Seed tuples are fixed in the controller
and are part of every contract. There is no ambient random seed.

When a worker receives peer guidance, only its one-part random queue changes
from `ConstPrio` to `PreferWatchlist`; its dominant queue and FIFO queue are
unchanged. This is the minimal shared guidance channel.

## Arms and equal configured budgets

Each policy has four workers and at most 16 configured soft CPU-seconds per
coordinate.

- `independent_equal`: four uninterrupted four-second workers.
- `independent_unequal`: four uninterrupted workers with budgets 7, 4, 3,
  and 2 seconds. Train-only solo solve count assigns the longest budget;
  ties use lower median solve CPU and then worker index. The selected mapping
  is hash-pinned before validation.
- `restart_control`: checkpoints at 128 and 512 processed clauses, each also
  bounded by one soft CPU-second, followed by a fresh two-second continuation.
  No clauses are exchanged.
- `share_4`, `share_16`, and `share_64`: the same two checkpoint/restart
  boundaries as `restart_control`, with at most the named number of
  peer-produced watchlist clauses per continuation.

The two probe waves plus final continuation have per-worker configured soft
limits 1 + 1 + 2 seconds. A checkpoint may finish earlier. Restart overhead,
wrapper construction, and exchange are part of candidate wall latency but not
prover CPU. Every arm has the same four-core reservation and configured
aggregate prover CPU ceiling. The restart-only arm isolates the effect of
sharing from restart loss.

## Clause exchange and safety

Probe workers run with:

```text
--processed-clauses-limit=128   # first wave
--processed-clauses-limit=512   # second wave
--soft-cpu-limit=1
--print-saturated=eig
--print-sat-info
--tstp-out
```

Only complete `cnf` records from the processed positive-unit,
negative-unit, and non-unit sets are candidates. The controller:

1. removes exact bodies already present in a separately hashed canonical CNF
   snapshot of the original problem;
2. removes the empty clause and malformed records;
3. deduplicates by whitespace-normalized clause body;
4. requires a candidate to come from a different worker than its recipient;
5. orders by literal count, printed symbol count, proof depth, number of distinct peer
   producers (descending), body hash, and producer index; and
6. takes the first 4, 16, or 64 candidates.

Guidance is written as `cnf(..., watchlist, BODY).` in a wrapper which includes
the untouched target. `--static-watchlist` makes these clauses heuristic
targets only: they are not logical premises, are never deleted to trigger
termination, and cannot make an unsound proof.

The second wave shares first-wave clauses. The final continuation shares the
union of first- and second-wave peer clauses. This is bounded periodic
exchange, not mutation of a live Rust `ProofState`.

## Shared-preprocessing audit

For each problem, the controller separately runs canonical `--cnf` conversion
four times and once, records hashes and CPU/wall/RSS, and reports the observed
redundant preprocessing cost as an upper bound on possible process sharing.

The canonical CNF is used only to identify novel exchange candidates. Search
workers continue from the original problem. A shared-CNF search arm is not
allowed unless its winning proof can be replayed from the untouched original
problem; treating generated CNF clauses as fresh axioms would violate the
proof-reconstruction gate.

## Phases and leakage controls

Train performs:

- the shared-preprocessing audit;
- solo four-second runs for the four frozen workers; and
- one repetition of all six portfolio arms.

The unequal budget mapping is selected from solo training results and written
to a content-hashed `selection.json`. Validation requires that selection.
Test additionally requires the exact validation analysis hash. Validation and
test each run all six arms twice. Results resume only after the contract and
all referenced artifact hashes verify.

The expected class, source bytes, include files, corpus identity, binary,
ProofCheck executable, validation gate, scripts, preregistration, CPU list,
strategies, seeds, budgets, checkpoint counts, and selection are contract
bound.

## Proof and correctness gates

The primary race runs without proof rendering to avoid forcing every
resource-limited worker to serialize a large derivation. Every proof-status
winner is immediately rerun alone with the exact worker, input wrapper,
checkpoint/budget, seed, and proof options:

```text
--tstp-out --proof-object=1 --force-deriv=2
```

The replay must reproduce the proof class and produce a nonempty annotated
TSTP protocol.
The repository validation gate and ProofCheck 1.0 must accept the proof against
the untouched original target. Cooperative proofs are also checked against
their wrapper. Watchlist records may not occur as logical proof premises.

Correctness fails on a missing/mismatched source or include hash, unexpected
status polarity, satisfiable/counter-satisfiable status, missing proof replay,
failed ProofCheck result, malformed exchange record, self-only exchange,
configured-budget violation, surviving process, temporary-file residue,
contract mismatch, or unstable claimed solve across the two held-out
repetitions.

## Measurements

Validation and test are reported separately:

- reproducible and one-repeat solve sets per arm;
- arm-only and lost solves versus `independent_equal`,
  `independent_unequal`, and `restart_control`;
- winning worker, wave, seed, and proof hash;
- total child user/system CPU, wall latency, core utilization, and peak
  aggregate RSS per coordinate;
- cancelled/crashed worker counts and cleanup latency;
- probe clauses parsed, novel clauses, peer coverage, exchanged clauses,
  watchlist hits when emitted, and wrapper bytes;
- common-solve CPU, wall, RSS, and proof-size ratios;
- preprocessing hash reproducibility and one-versus-four cost;
- repeat stability and every correctness failure.

## Frozen decision

`adopt` a sharing cap only if all correctness gates pass, validation and test
both lose no reproducible solve versus all three controls, and either:

1. it has at least one reproducible test solve absent from all three controls;
   or
2. validation and test each have at least four common solved repetition
   coordinates versus `restart_control`, with median total CPU and wall ratios
   at most `0.95` and median peak-RSS ratio at most `1.05`.

If multiple caps qualify, choose the most test-only solves, then lowest test
median CPU ratio, then the smaller cap.

`stop` if correctness fails, any reproducible test solve is lost, or complete
validation and test show no unique solve and no qualifying efficiency signal.
Fewer than four common solved coordinates, a one-repeat-only delta, or
validation/test disagreement is `uncertain`.

Independent unequal slicing advances over equal slicing under the same
no-loss/unique-solve or paired-efficiency rule. Shared preprocessing advances
only if its measured redundant cost is at least 5% of equal-portfolio CPU and
a reconstructable original-problem proof path is demonstrated.

No production scheduler or default changes unless an arm reaches `adopt`.
