# Preregistration: cooperative ground-theory SMT checking

Bead: `E_Rust_Port-9jt.5.7`

Date frozen: 2026-07-30

## Question

Can a proof-aware, ground-only SMT layer prune inconsistent arithmetic branches
or close split workloads with acceptable overhead, while keeping every accepted
theory step independently replayable and preserving a zero-dependency fallback?

A negative result closes this evaluation Bead when all reporting and
falsification gates pass. It does not require a production integration.

## Protocol and trust boundary

The eligible fragment is typed ground difference logic over `$int` or `$real`.
Each constraint has the canonical form `x - y <= c`, where `x` or `y` may be
the distinguished zero term and `c` is an exact rational. Integer workloads
require integral bounds. A workload asserts a shared base once, then checks
each branch under `push`/`pop`.

The protocol is deliberately smaller than SMT-LIB and maps unambiguously to the
typed arithmetic frontend established by `E_Rust_Port-9jt.5.3` and
`E_Rust_Port-9jt.5.9`. It excludes mixed sorts, quantifiers, nonlinear terms,
strict inequalities, division, rounding, arrays, bit-vectors, and
solver-specific extensions.

For an eligible result:

- `unsat` is accepted only when the named core independently contains a
  negative cycle under exact rational Bellman-Ford replay;
- `sat` is accepted only when every asserted constraint holds in an exact
  parsed model; and
- malformed, missing, unsupported, timed-out, interrupted, or unverifiable
  evidence becomes `Unknown`.

The tracked Python verifier and separately compiled dependency-free Rust replay
checker share only the canonical protocol, not Z3 APIs or output-classification
code. The Rust checker is the prototype for "independently checkable by
Umlaut"; it must accept every counted decision and reject seeded certificate
mutations. A matching second-solver status is not proof evidence.

## Frozen corpus

`build_corpus.py` deterministically generates the tracked `corpus.json` and
`--check` requires byte-for-byte identity. The corpus is frozen before any
candidate execution.

Each train, validation, and test partition contains both integer and real
families with:

- an all-inconsistent workload that can be closed;
- a mixed workload with both prunable and feasible branches;
- an all-feasible workload;
- a neutral workload that must bypass both SMT backends; and
- a general-linear workload outside the replay fragment whose raw solver
  answer must not be trusted.

The partitions use increasing cycle sizes of 4, 8, and 16 variables. Candidate
selection or thresholds may use train only. Validation and test are held out
for the preregistered decision.

## Fixed variants and repetitions

All variants traverse the same workload and branch order.

1. `no_smt` performs no theory decision and returns `Unknown`.
2. `process` uses one pinned Z3 executable through a shell-free persistent
   SMT-LIB process. Each workload reuses its base with `push`/`pop`.
3. `ffi` uses one pinned `libz3.so` through an experiment-only Rust C API
   driver. It uses one reference-counted context and incremental solver per
   workload.

Both Z3 variants use a 5,000 ms solver timeout, deterministic seeds, named
assertions, unsatisfiable cores, and models. Five measured repetitions follow
one warm-up. Verdicts and normalized evidence must be deterministic across
repetitions; timing is summarized by median and p95.

The no-SMT and neutral dispatch paths are timed in the Python harness. Solver
timing includes check plus evidence extraction. Process totals additionally
report executable startup/shutdown; FFI totals report driver startup/shutdown
separately from in-process calls.

## Correctness and falsification gates

Before interpreting performance, all of the following must pass:

1. deterministic corpus, renderer, exact-number, core, model, protocol, and
   malformed-evidence tests;
2. pinned Z3 source identity and binary/library hashes;
3. process and FFI raw verdict agreement on every common branch;
4. expected verdict agreement on every frozen supported branch;
5. exact Python evidence validation for every counted `sat` or `unsat`;
6. dependency-free Rust replay acceptance for every counted certificate;
7. Rust replay rejection after removing a core member, altering a core bound,
   corrupting a model, and flipping a decision;
8. unsupported general-linear raw answers rejected as trusted steps;
9. process cancellation terminates within one second without a counted result;
10. C API interruption returns `unknown` within one second without a counted
    result; and
11. a missing executable, malformed output, timeout, or driver failure is
    classified as `Unknown`/error and never as a proof result.

Failure of a gate makes the relevant backend inconclusive and forbids
production adoption.

## Measurements

The report records, by backend, partition, sort, and cohort:

- raw and trusted `sat`/`unsat`/`unknown` counts;
- independently verified cores and models;
- pruned branches and completely closed all-inconsistent workloads;
- unsupported raw decisions rejected from the trusted stream;
- call, startup, shutdown, median, p95, and aggregate wall time;
- deterministic verdict/core/model hashes across repetitions;
- timeout and cancellation/interruption outcome and latency;
- Z3 executable, shared library, FFI driver, replay driver, and build-tree
  sizes and hashes;
- dynamic library dependencies and candidate runtime-byte deltas; and
- `Rust replay verified / raw SMT decisions` plus
  `Rust replay verified / trusted SMT decisions`.

Neutral work must incur no solver call. Package measurements distinguish an
external executable candidate from a shared-library FFI candidate; neither is
added to the default package.

## Advancement rule

A production follow-up is justified only when, on combined held-out validation
and test data:

1. every correctness and falsification gate passes;
2. both SMT backends lose no expected decision and accept no unsupported step;
3. 100% of trusted SMT-derived steps pass Rust replay;
4. at least four all-inconsistent workloads are closed and at least 20 branches
   are pruned;
5. p95 trusted-call latency is at most 2 ms for FFI and at most 10 ms for the
   persistent process backend;
6. neutral workloads make zero solver calls;
7. cancellation/interruption is bounded by one second; and
8. packaging, unsafe-FFI, version-pinning, and deployment evidence identifies
   no unresolved blocker.

If correctness passes but packaging or trust blocks adoption, close this
evaluation with production unchanged and file a narrower follow-up only when
the evidence supports one. No post-hoc corpus or threshold changes this rule.
