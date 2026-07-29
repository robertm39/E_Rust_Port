# Preregistration

## Question

Which current Umlaut frontend phase dominates parsing, clausification, and
clausal preprocessing across representative CNF, FOF, TFF, and THF inputs, and
does a profile expose one safe specialization worth prototyping?

This experiment may conclude that no production change is justified. That is a
valid result if the frozen measurements and go/no-go rules below are applied
without tuning them after the run.

## Frozen implementation and reference

The baseline is the repository snapshot created immediately after this file and
the harness are added, before inspecting any timing, DHAT, or Callgrind result.
The comparison reference is E commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`, built by the repository's
`linux_compat.py` helper. FOL E is used for CNF, FOF, and TFF; `eprover-ho` is
used for THF. Reference results provide context, not an adoption threshold.

All prover builds, executions, and profiling run sequentially on one ephemeral
Ubuntu 24.04 Linode. Local execution is limited to controller unit tests and
result analysis.

## Deterministic corpus

`frontend_profile.py generate` creates three sizes (1,000, 10,000, and 50,000
annotated formulas) for each of four dialects:

- CNF clauses with one positive, one negative, and one binary literal;
- quantified FOF implications;
- typed first-order TFF implications; and
- typed higher-order THF applications and implications.

Names are unique, while term and predicate symbols cycle over a fixed set.
This avoids a single repeated owner without turning the study into another
unique-signature scaling benchmark. The report records every generated file's
byte length and SHA-256.

The 10,000-record stratum is held out from hotspot selection. The 1,000 and
50,000 strata select the dominant phase; the 10,000 stratum is then used for
DHAT and Callgrind. This is a diagnostic holdout, not a statistical train/test
claim.

## Timing methods

GNU `/usr/bin/time` records wall seconds, user CPU seconds, system CPU seconds,
maximum RSS in KiB, and exit status. Each timed command is run five times in an
interleaved deterministic order after one untimed warm-up. Startup is measured
with `--version`. The three frontend modes are:

1. `--syntax-only`;
2. `--cnf --no-preprocessing`; and
3. `--cnf`.

All timed output is redirected to `/dev/null`. For each implementation,
dialect, size, and mode, the analysis reports the median wall, CPU, and peak
RSS. Nonzero exit status or an incomplete repetition set invalidates the run.

Phase estimates subtract medians:

- parse = syntax-only minus startup;
- clausification = CNF/no-preprocessing minus syntax-only; and
- clausal preprocessing = full CNF minus CNF/no-preprocessing.

Negative differences caused by timer noise are reported and clamped to zero
only for phase-fraction calculations. Total frontend time is full CNF minus
startup.

## Allocation and instruction profiles

After timing selects a held-out dialect, DHAT runs syntax-only,
CNF/no-preprocessing, and full CNF on its 10,000-record file. The retained
report includes total allocated bytes and blocks, peak live bytes and blocks,
end-live bytes and blocks, wall time, and maximum RSS.

Callgrind then profiles the single phase endpoint that bounds the dominant
increment. `callgrind_annotate --inclusive=yes` is retained verbatim, and the
analysis records the leading named functions. Debug information is allowed in
the optimized profile build, but optimization level and source are unchanged.

## Correctness and origin gates

Before profiling:

1. controller unit tests pass;
2. every generated corpus parses successfully in its assigned baseline and C
   reference binary;
3. all timed commands exit zero;
4. the baseline produces deterministic byte-identical TSTP CNF output at
   output level 4 in two executions of each 1,000-record corpus; and
5. generated CNF names and `inference(...)` records in that output are
   inventoried so a later prototype cannot silently discard proof origins.

If a prototype is built, its syntax-only and both CNF modes must exit exactly
like baseline on every corpus. Its TSTP CNF output, including source names and
inference ancestry, must be byte-identical to baseline for all 1,000-record
and held-out 10,000-record files.

## Prototype go/no-go rule

At most one production prototype is permitted. It is attempted only if all of
the following hold:

1. one phase is at least 50% of measured frontend wall time in at least two
   dialect/size strata;
2. that phase is at least 25 ms in a 50,000-record stratum;
3. the held-out DHAT or Callgrind result names one source-level hotspot that
   plausibly explains the phase; and
4. the specialization can preserve parser acceptance, clausification output,
   and proof-origin construction without unsafe code or a new dependency.

Otherwise production remains unchanged and the experiment closes as a
profiling result.

## Prototype adoption rule

A prototype is adopted only if:

1. every correctness and origin gate passes;
2. the held-out dominant-phase wall median improves by at least 15%;
3. held-out total frontend wall median improves by at least 10%;
4. total allocated bytes and peak live bytes do not regress by more than 2%;
5. maximum RSS does not regress by more than 2%;
6. the standard main-executable differential has no new mismatch;
7. the standard support-tool differential has no new mismatch;
8. the ten-case solve benchmark has identical statuses and proof/output hashes,
   with no paired median wall regression greater than 2%; and
9. repository formatting, tests, strict Clippy, and release builds pass.

Failure of any rule means the prototype is reverted. Thresholds are not changed
after results are observed.

