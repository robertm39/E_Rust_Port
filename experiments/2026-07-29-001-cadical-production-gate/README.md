# Optional CaDiCaL production gate

Bead: `E_Rust_Port-9jt.4.8`

Status: preregistered before capture or timing.

## Question

Does the production `IncrementalSatService` boundary preserve its correctness
contract and justify either of the already proposed 128- or 256-clause
automatic dispatch thresholds on a workload that was unavailable to the
backend bake-off?

## Frozen inputs

- CaDiCaL 3.0.1 revision
  `c60730422e758ef1cebe7aeddf2dda31c996bf04`.
- Upstream archive SHA-256
  `ad639a302b7c4cb4a24f37b7cd0cf7533674e6069c20a561505bccef1c2b4444`.
- CASC-30 corpus and immutable manifest documented in
  `docs/casc-benchmark-matrix.md`.
- SATCheck source families `ALG`, `GEO`, `ITP`, `LAT`, `NLP`, and `SCT`.
  None occurs in any selection from experiment
  `2026-07-28-012-incremental-sat-service`.
- AVATAR-style seed `20260729`, sizes frozen by
  `generate_avatar_workloads.py`.
- Dispatch thresholds: exactly 128 and 256 clauses. No other threshold will be
  timed or inferred from these results.
- Five measured repetitions after one unmeasured warm-up, one pinned CPU, and
  seed `20260729`.

The family selector admits at most six problems per named family, requires a
500--250,000 byte problem in the first-order/EPR/UEQ categories, and ranks
eligible problems by a salted hash. The checked-in `fresh-selection.jsonl` is
the immutable result. Capture shape is not an input to selection.

## Workloads

The first class captures the exact post-grounding, post-pure-filter CNF passed
to production SATCheck. `instrument_capture.py` adds only an experiment
recorder to a remote source copy of the current service path, with exact
single-anchor assertions. Every selected problem runs for three CPU seconds,
with up to eight captures.

The second class is a deterministic AVATAR-style abstraction workload. Each
session contains selector-guarded pigeonhole components. Per-call assumptions
activate and deactivate components, including repeated UNSAT queries that
exercise incremental learning and failed-assumption cores. Clause counts
straddle both frozen thresholds. This is an abstraction-shaped service
workload, not a claim that Umlaut already has a production AVATAR loop.

`service_probe.rs` is copied temporarily into `src/bin/` on the remote
experiment workspace. It uses the production Rust service and production
CaDiCaL wrapper; it does not use the earlier C++ bake-off adapter.

## Correctness and decision rule

Every internal/CaDiCaL query must agree on SAT, UNSAT, or Unknown. SAT models
and UNSAT failed cores are already validated inside the production boundary;
any service error, process failure, status mismatch, invalid model, invalid
core, or missing workload class is a hard rejection.

For a threshold to pass:

1. it must dispatch at least one session in each workload class;
2. combined insertion-plus-query cost must improve by at least 25% versus the
   all-internal policy;
3. combined query p95 must improve by at least 25%;
4. neither workload class may regress aggregate cost by more than 10%; and
5. its combined aggregate cost must beat the other frozen threshold by at
   least 5%.

If neither threshold passes, automatic dispatch remains nondefault. If one
passes, the result permits but does not require making it the default; a
synthetic abstraction workload or sparse production capture may still justify
the more conservative opt-in decision.

## Reproduction

All Rust/C build, prover capture, and timing commands run on the mandatory
Ubuntu 24.04 Linode. The result archive is collected under ignored
`.artifacts/experiments/2026-07-29-001-cadical-production-gate/`.
`FINDINGS.md` records the exact run contract, commands, hashes, and decision.
