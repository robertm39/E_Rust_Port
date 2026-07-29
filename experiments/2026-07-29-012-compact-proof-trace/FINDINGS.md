# Compact proof-trace storage and reconstruction

## Status and decision

Completed for Bead `E_Rust_Port-9jt.8.2`.

The framed output log is technically viable: it reconstructs every sampled
proof byte-for-byte, passes independent checking, compresses the aggregate
solution stream to 4.34% of its original size, adds less than 3 ms median
spooled replay latency on the largest proof, and fails closed under corruption
and interruption.

It is not integrated into production. The measured 35 MB LUSK6 owner is live
semantic derivation state used during search, whereas this codec starts after
TSTP materialization. Replacing one with the other would either retain both
representations or delete clause/formula bodies and parent identities that
runtime consumers still need. The experiment therefore passes its output-log
gate but rejects the production-integration gate.

## Frozen boundary

The final Ubuntu 24.04 run used:

- source commit `970353d0014792395eb0eb48a834b0f651750e52`;
- uploaded snapshot SHA-256
  `49efed6f0a13c9e9e68612bfc5874eebc4e94ce9a64c5ec3cb5df37b22bc3318`;
- release Umlaut SHA-256
  `d34912867aad36a621cd83fb9ed19c774e4f184d9be3f70fd3cd91da2556aa8a`;
- Linode run `260729-164307-f3c0` in `us-iad`; and
- ProofCheck 1.0 executable SHA-256
  `92bb5193a9d8b2857fb97d9bd9fb6f16f5bcb57d07e4307d7f087e403ff51c7e`.

ProofCheck passed its complete 117-test self-certification before any sampled
proof was accepted. TFF and THF were deliberately excluded because this
checker boundary does not independently verify them.

## Representation and reconstruction

The experiment codec stores the exact emitted byte stream in versioned,
immutable frames. Frames normally aggregate complete lines up to 64 KiB; a
single longer line remains one bounded frame. Each frame records:

- raw and stored lengths;
- whether the payload is raw or zlib-compressed; and
- CRC-32 over the reconstructed bytes.

The trailer records frame count, total uncompressed bytes, and whole-stream
SHA-256. Replay validates every boundary and checksum before atomically
publishing the destination. The compact variant retains the frames in memory;
the spooled variant streams the same bytes through a regular file and never
retains the complete reconstructed proof before publication.

This is intentionally an output-byte log rather than a semantic TPTP parser.
It cannot change formulas, inference records, line endings, comments, or final
newline state during replay.

## Storage and bytes per derived record

Here “derived record” means an emitted FOF/CNF TSTP formula with
`inference(...)` ancestry. It is an output-proof metric, not the number of all
clauses generated during saturation.

| Proof | Derived records | Eager bytes | Compact/spool bytes | Eager bytes/derived | Compact bytes/derived | Ratio |
|---|---:|---:|---:|---:|---:|---:|
| Small FOF Theorem | 6 | 1,334 | 676 | 222.33 | 112.67 | 50.67% |
| Nontrivial CNF Unsatisfiable | 3 | 1,386 | 656 | 462.00 | 218.67 | 47.33% |
| `COL003-19` | 12 | 12,887 | 1,971 | 1,073.92 | 164.25 | 15.29% |
| `SYN846-1` | 149 | 864,603 | 34,872 | 5,802.70 | 234.04 | 4.03% |
| **Aggregate** | **170** | **880,210** | **38,175** | **5,177.71** | **224.56** | **4.34%** |

The compact and spooled files are identical by construction. The aggregate
passes the preregistered 70% size gate by a wide margin.

## Output latency

Each operation ran 25 times in a deterministic interleaving. The table reports
medians in milliseconds.

| Proof | Compact encode | Compact replay | Spool encode | Spool replay | Prover wall |
|---|---:|---:|---:|---:|---:|
| Small FOF Theorem | 0.095 | 0.108 | 0.148 | 0.602 | 2.555 |
| Nontrivial CNF Unsatisfiable | 0.115 | 0.108 | 0.140 | 0.598 | 2.596 |
| `COL003-19` | 0.425 | 0.145 | 0.482 | 0.612 | 33.416 |
| `SYN846-1` | 19.738 | 1.923 | 19.947 | 2.864 | 592.616 |

Every replay is below the frozen limit of 100 ms or 25% of the corresponding
prover wall time, whichever is larger. Encoding is also small relative to the
largest proof's end-to-end run, but it was not part of the adoption threshold.

## Peak memory

GNU `time -v` isolated each worker in a fresh Python process. The no-payload
control used 19,712 KiB RSS, so small-proof deltas are dominated by interpreter
and page-level noise and are retained without clamping.

For the largest proof:

| Worker | Maximum RSS | Delta from no-payload |
|---|---:|---:|
| Eager retain | 20,608 KiB | +896 KiB |
| Compact retain | 20,096 KiB | +384 KiB |
| Compact in-memory replay | 21,376 KiB | +1,664 KiB |
| Spooled replay | 20,096 KiB | +384 KiB |

Spooled replay saves 512 KiB of payload-adjusted RSS versus eager retention and
passes the preregistered non-positive-delta gate. In-memory replay is
intentionally worse because it holds both compressed and reconstructed byte
strings; it is not the memory-pressure choice.

These worker results must not be substituted for a full-search measurement.
The carried-forward LUSK6 Massif peak remains 197,700,288 total bytes, with
35,000,832 useful heap bytes attributed to rewrite-derivation stacks. No
integrated candidate changed that peak.

## Independent reconstruction checks

For each of the four proofs:

1. the original output received `VerifiedGood`;
2. the compact reconstruction received `VerifiedGood`;
3. the spooled reconstruction received `VerifiedGood`;
4. a second compact replay and second spooled replay reproduced the original
   whole-file SHA-256; and
5. prover exit status and final SZS status matched the frozen expectation.

This is 12 positive external checks: three independently checked byte streams
for each of four problems. The exact hashes are retained in the raw report.

## Failure recovery

Every proof log was tested with:

- truncation at the midpoint of the final frame;
- one payload bit flip; and
- an over-limit declared frame length.

All 12 mutations were rejected at a named frame. None published a final file,
and all recoverable error paths removed their temporary file.

Each valid spool was also replayed in a child process paused after its first
frame and then terminated with `SIGTERM`. All four interruptions left the final
path absent. The expected single process-owned temporary file was detected,
the next complete replay atomically published the exact original SHA-256, and
the harness then removed the orphan.

## Why production remains unchanged

The code audit found live derivation consumers outside final rendering:

- proof-state graph closure and parent extraction;
- priority functions that inspect derivation operation codes;
- split-definition parent remapping, which mutates stored formula references;
- dummy-quote, evaluation-GC, and initial-clause classification; and
- rewrite ancestry based on stable clause generations and demodulator handles.

The archived clause/formula objects also supply proof bodies that are absent
from a byte log until after rendering. A post-render spool therefore cannot
release the live `PStack<DerivationEntry>` owner. Adding it to current Umlaut
would retain both states and add zlib/framing code to a proof-publication path
that already publishes atomically.

A true live semantic arena would require migrating all of those consumers,
measuring the full LUSK6 peak, and passing the stricter production gates in the
preregistration. That larger ownership change is not justified by this
post-render result, especially after the prior 24-byte entry experiment
preserved proof counts but regressed exact LUSK6 instructions by 12.118880%.

## Validation and evidence

- local controller codec tests: 7 passed;
- Ubuntu controller codec tests: 7 passed;
- ProofCheck self-certification: 117 passed;
- sampled external proof checks: 12 `VerifiedGood`;
- malformed-log checks: 12 rejected, zero publications;
- forced-interruption checks: 4 final paths withheld, 4 exact recovery replays;
- Markdown link checker: 269 files;
- production Rust source: unchanged.

The ignored evidence archive is
`.artifacts/experiments/2026-07-29-012-compact-proof-trace/evidence.tar.gz`,
372,570 bytes, SHA-256
`469d8d1819e33057ae8b76988511bb2f032c9f7394ce117fa0942362d96be09a`.
Its `report.json` SHA-256 is
`8e00eefd4100408aa21cee74368327fee9f49ca4b607e6235370fd251a0eddd7`.

Both the Linode and its firewall were deleted after evidence collection.
