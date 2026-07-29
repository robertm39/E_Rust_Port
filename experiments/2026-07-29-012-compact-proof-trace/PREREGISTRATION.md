# Preregistration

## Question

Can an immutable, checksummed, independently replayable proof log reduce the
bytes retained for final proof publication without weakening Umlaut's current
first-order proof guarantees, and does that result justify replacing the live
per-clause derivation stacks used during saturation?

This experiment may reject production integration. A compact post-search
artifact is not evidence that the live search derivation can be deleted:
proof-graph traversal still needs stable clause/formula parents, and rendering
still needs archived formula and clause bodies.

## Frozen baseline and prior evidence

The baseline source is commit
`8ecb43181d5c3c01365b03037ce932e65673692f`. Experiment
`2026-07-27-003-memory-representation-profile` measured a 197,700,288-byte
LUSK6 Massif peak, including 35,000,832 useful heap bytes retained through the
rewrite-derivation caller tree. `DerivationEntry` remains 32 bytes.

Experiment `2026-07-23-011-packed-clause-derivation-ref` already rejected
shrinking `DerivationEntry` from 32 to 24 bytes: exact LUSK6 instructions
regressed by 12.118880% despite unchanged proof-search counts. This experiment
will not repack clause identities or change that production type.

All prover builds, executions, proof checks, timing, and memory measurements
run serially on one ephemeral Ubuntu 24.04 Linode. Local execution is limited
to controller tests, static inspection, and result analysis.

## Frozen proof corpus

The positive-only corpus contains these current first-order proofs:

1. the small FOF Theorem fixture `fof_theorem.p`;
2. the nontrivial CNF Unsatisfiable fixture `cnf_unsatisfiable.p`;
3. held-out FOF `COL003-19.p`; and
4. held-out FOF `SYN846-1.p`.

The first two fixtures come from experiment `2026-07-27-004`; the held-out
problems use the immutable TPTP snapshot already used by experiment
`2026-07-29-009`. Every emitted baseline and reconstructed solution must receive
external `VerifiedGood` from integrity-pinned ProofCheck 1.0. TFF and THF are
excluded because the adopted checker boundary does not independently verify
them.

Each prover invocation uses `--auto --tstp-out --proof-object=1` with the
experiment's frozen CPU limit. Exit status, final SZS status, complete stdout
SHA-256, and complete stderr SHA-256 are recorded.

## Three storage variants

The experiment compares the following representations of the exact emitted
solution stream:

1. **Eager**: the original proof bytes retained as one in-memory byte string.
2. **Compact immutable**: a versioned binary log containing independently
   compressed frames. Each input line is one immutable frame with its
   uncompressed length, compressed length, and CRC-32. Reconstruction validates
   all framing, lengths, decompression boundaries, and checksums before
   publishing any bytes.
3. **Streamed/spooled**: the same binary frames written incrementally to a
   regular file and replayed incrementally to a temporary output file. The
   final path is atomically replaced only after the complete log validates.

The format uses only Python's standard-library `zlib`; it introduces no Umlaut
runtime dependency and stores the original bytes rather than reparsing TPTP.
The codec must preserve empty lines and final-newline state exactly. Identical
input bytes must produce identical log bytes.

This framing intentionally evaluates the safest output-log boundary first. It
does not claim to be a compact semantic inference arena and cannot by itself
release live clauses, formulas, terms, or derivation parents during search.

## Metrics

For every proof, the report records:

- original, compact-log, and spool-file bytes;
- the number of emitted TSTP formula records and the subset with
  `inference(...)` ancestry;
- bytes per inferred record for every storage variant;
- median encode and reconstruction latency over 25 interleaved repetitions;
- complete reconstructed SHA-256 and deterministic second-replay SHA-256;
- ProofCheck verdict and wall time for the original and both reconstructed
  outputs; and
- maximum resident set size for isolated eager-retain, compact-retain,
  compact-replay, and spooled-replay workers.

Because Python interpreter residency can dominate small proofs, the memory
report also includes a no-payload worker control. RSS deltas are reported but
negative/noisy deltas are not clamped.

The study carries forward the measured full-search LUSK6 peak and
rewrite-derivation attribution. It will clearly separate those production
search measurements from codec-worker RSS; it will not project a new
full-search peak from compressed final-output bytes.

## Failure recovery

For each compact log, the harness creates:

- one truncation at the midpoint of the final frame;
- one payload bit flip; and
- one invalid length header.

Every mutation must fail closed, identify the malformed frame, leave no final
published output, and remove its temporary output. The unmodified spool must
survive a forced reconstruction-process interruption: no final path may be
visible until a subsequent complete replay succeeds.

## Correctness gates

The experiment is invalid unless:

1. ProofCheck 1.0 passes all 117 self-certification tests and matches the pinned
   archive and executable SHA-256 values;
2. all four original proofs are `VerifiedGood`;
3. both reconstructions of every proof are byte-for-byte identical to the
   original;
4. compact encoding is deterministic;
5. every reconstructed proof is independently `VerifiedGood`;
6. every malformed log is rejected without publishing partial output; and
7. all controller tests pass on local Python and Ubuntu Python 3.12.

## Output-log decision rule

The compact/spooled output boundary is considered technically viable only if
all correctness gates pass and the aggregate compact log is no larger than
70% of the aggregate eager proof bytes. Replay may not take more than 100 ms or
25% of the corresponding prover proof-publication wall time, whichever is
larger. Spooling must have a non-positive payload-adjusted RSS delta relative
to eager retention on the largest proof.

Failure rejects the codec boundary.

## Production-integration decision rule

No production source change is permitted from output-codec results alone.
Replacing the live clause derivation representation would require a separate
integrated candidate that:

1. reconstructs all current proof consumers, including runtime priority and
   split-parent inspection;
2. preserves exact search counts and independently verified proof output;
3. reduces measured LUSK6 peak RSS by at least 10%;
4. reduces the measured 35,000,832-byte derivation owner by at least 25%;
5. regresses neither exact LUSK6 instructions nor ten-case paired wall medians
   by more than 2%; and
6. provides deterministic recovery from an unavailable or corrupted log.

If the compact byte log cannot release the live semantic owners identified in
the code audit, production remains unchanged and the findings must say so even
when the output-log gate passes.
