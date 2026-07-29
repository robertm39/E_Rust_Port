# Preregistration

Date: 2026-07-29

Bead: `E_Rust_Port-9jt.7.4`

Baseline source: commit `78059361` plus this preregistration commit. The
benchmark binaries will be built from the later instrumentation commit; their
source commit and SHA-256 digests must be recorded in the run contract.

## Question and hypothesis

Umlaut already implements E-shaped perfectly shared rewrite links and two
normal-form dates on canonical terms. The primary hypothesis is that these
positive and negative cache records avoid enough repeated normalization work
to improve end-to-end proof search without changing logical results or proof
reconstruction.

The falsifying hypothesis is that the cache has a low hit rate, negligible
end-to-end effect, or unacceptable retained-memory cost on both a frozen
CASC-aligned sample and established rewrite-heavy workloads.

## Candidate and control

The candidate is the normal production build:

- top-level rewrite links are followed;
- structural links reuse normalized subterms;
- rule/full normal-form dates skip searches against unchanged demodulator
  epochs; and
- rewrite chains remain available for proof ancestry reconstruction.

The control is a Cargo feature named `rewrite-cache-ablation`. It is an
experiment-only build mode, disabled by default. At each normalization entry
it ignores a previously shared rewrite link and the normal-form-date fast
return, recomputes from the original canonical term, and installs a fresh
rewrite chain during that call. The fresh chain must remain long enough for
the unchanged proof-trace reconstruction path to consume it. The control is
invalid if focused tests or proof validation fail.

This is a full-cache-versus-recompute ablation. It does not claim that a
production cache could safely omit every metadata field, so maximum RSS and
term-bank storage are conservative memory comparisons rather than a precise
byte price for the cache.

## Instrumentation

Stable opt-in search telemetry will add per-run deltas for:

- rewrite-cache lookups;
- lookups that find an eligible shared link;
- rewrite-link edges followed;
- normal-form-date checks;
- normal-form-date hits; and
- uncached rewrite links created.

Derived measures are:

- link hit rate = link hits / link lookups;
- mean followed path = edges followed / link hits;
- normal-form-date hit rate = date hits / date checks;
- cached rewrite fraction = max(0, rewrite steps - uncached links) /
  rewrite steps; and
- saved traversal proxy = followed link edges + normal-form-date hits.

The saved-traversal value is explicitly a proxy, not an instruction count.
CPU time, generated/processed clauses, high-water clauses, maximum resident
pages, and term-bank storage provide the end-to-end and memory evidence.

## Correctness tests

Before benchmarking, focused Linux tests must cover:

1. a shared term reused under an unchanged demodulator epoch;
2. a later rule addition invalidating an old negative normal-form result;
3. a later rule extending an existing rewrite target to a new normal form;
4. restricted versus unrestricted rewrite links;
5. exact rewrite-demodulator ancestry after cached reuse;
6. exact ancestry under `rewrite-cache-ablation`; and
7. telemetry counters and per-run delta rendering.

The optimized all-feature build, the ablation build, Rustfmt, Clippy, and the
full locked test suite must pass on Ubuntu before any performance result is
accepted.

## Frozen workloads

### CASC-aligned test

`corpus.json` freezes the same candidate-blind 20-problem CASC-30 test split
used by experiments `2026-07-28-008` and `2026-07-29-013`: six FEQ, six FNE,
two EPS, and six UEQ problems. The recorded content hashes must match the
pinned manifest with SHA-256
`31c9a99e4b34b56352b3311f3efe5c97f728fd078783085e1811d83eec271f6d`.

Both builds run two repetitions at:

- short: 5 second soft / 7 second hard CPU; and
- larger: 20 second soft / 23 second hard CPU.

The common fixed strategy is
`(5*Refinedweight(ConstPrio,2,1,1.5,1.1,1.1),1*FIFOWeight(ConstPrio))`,
KBO6, and full forward demodulation. Proof objects are enabled.

### Rewrite-heavy falsification set

The following repository-local workloads were selected from findings that
predate this experiment, before observing either candidate:

- `eprover/EXAMPLE_PROBLEMS/TPTP/COL042-8.p`;
- `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop`;
- `eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6ext.lop`;
- `eprover/EXAMPLE_PROBLEMS/TPTP/SWC078-1.p`; and
- `eprover/EXAMPLE_PROBLEMS/TPTP/HEN011-2.p`.

Both builds run two repetitions with a 30 second soft / 33 second hard CPU
budget and the same fixed strategy. Repository file hashes must be captured
before execution.

## Execution and analysis

All Rust builds, tests, prover execution, and benchmarks run on one ephemeral
Ubuntu 24.04 Linode through `linode-runner.ps1`. Jobs are deterministically
interleaved by a hash of contract, problem, build, budget, and repetition,
with at most four concurrent workers and 1536 MiB per job. Run contracts,
stdout, stderr, telemetry, proofs, source revision, binary hashes, host
metadata, and exact commands are retained.

Results are paired by problem, budget, and repetition. Analysis reports each
build and workload separately, medians over paired coordinates, common-solved
medians, reproducible solve sets, status disagreements, missing or invalid
telemetry, and proof-validation coverage. Timeout-limited resource totals are
reported but are not treated as speedups.

At least one proof from every reproducibly solved category and build must be
checked with the same integrity-pinned ProofCheck 1.0 path used by experiment
`2026-07-29-013`, subject to its documented UEQ/Skolem adapters. If the frozen
sample yields no proof for a category, focused ancestry tests are the stated
coverage boundary.

## Decision rules

The production cache is retained unchanged if all correctness gates pass and
either:

- it has at least one reproducible larger-budget solve absent from the
  ablation, with no ablation-only solve or polarity disagreement; or
- over common solved larger-budget CASC coordinates its median CPU ratio
  (cache / ablation) is at most `0.95`, generated and high-water ratios are at
  most `1.02`, and maximum-RSS ratio is at most `1.05`.

A selective/hot-term follow-up is warranted, but no production policy changes
in this Bead, when correctness passes yet the full cache misses the retention
gate and at least one of these holds:

- combined link hit rate is below `0.10`;
- maximum-RSS or term-storage ratio exceeds `1.05`; or
- cache / ablation median CPU is above `1.02` on either the common-solved CASC
  set or the rewrite-heavy set.

Otherwise the result is neutral and the existing default remains in place for
compatibility. Any contradiction, proof-validation failure, nondeterministic
resume, binary/source mismatch, or invalid ablation makes the performance
comparison inconclusive and forbids a cache-policy change.

No post-hoc workload may affect the decision. Exploratory diagnostics must be
labeled separately.

