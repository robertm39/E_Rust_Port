# Detailed preservation decisions

## Status

Completed for the affirmative-decision subset of Beads
`E_Rust_Port-j76.4`. Of 1,327 detailed C `Change Later` records, 586 already
state a concrete Rust preserve/change/safe-ownership decision without any of
the audit's conservative incomplete, transitional, or future-port signals.
Fifteen had been closed by focused slices; this reconciliation reviewed the
remaining 571. One candidate (`E_Rust_Port-j76.4.1288`) was immediately
reopened because direct C/Rust comparison found that `TBInsertOpt` did not yet
apply C's higher-order `WHNF_deref` policy. Experiment 040 implements and
validates that behavior separately. The vendored C checkout remains unchanged.

## Question

Which detailed reviews already contain enough post-compatibility design
judgment to close without a new implementation, and which must remain for
manual semantic review?

## Method

[`audit_preservation_decisions.py`](audit_preservation_decisions.py) reuses the
conservative classifier from Experiment 330. A record qualifies only when:

1. its immutable legacy text contains an affirmative Rust decision such as
   preserve, mirror, retain, deliberately replace, avoid, or model;
2. it contains none of the conservative remaining, incomplete, temporary,
   fallback, provisional, or future-port signals;
3. its migrated identity and ordinal pass the original backlog audit; and
4. the final compatibility, performance, and validation evidence remains
   present.

The report does not use issue status to define the set, so it remains stable
after closure. [`audit-reference.json`](audit-reference.json) pins 586 records
across 181 source-unit files with digest
`8a89c6edcb735f05dc427a1fe789f5091e50d264a16038c6e3396e3f509a384c`.

The audit verifies all 583 standard migrated content hashes. The three
historical nonstandard records (`j76.4.662`, `.1326`, and `.1327`) do not carry
the standard hash envelope; their complete text still participates in the
aggregate digest. Exact legacy text remains in the current C-source docs for
506 records; the other decisions remain immutable in Beads after later
documentation consolidation.

## Decision

With the explicit `j76.4.1288` exception routed to Experiment 040, the other
records are resolved as documented compatibility/safety decisions. They do not
request code after compatibility; their purpose was to ensure that a port did
not accidentally copy raw-pointer lifetime, process-global state, undefined
behavior, allocation-sensitive ordering, or obsolete C API shape where Rust
already provides the recorded safe equivalent.

The closure is supported by the fresh compatibility milestone:

- all 50 main-prover and 216 support-tool cases have zero unexpected
  differences;
- the maintained main prover is `1.0801753448x` C, within the `1.10x` target;
- the latest full snapshot passes strict all-target/all-feature Clippy and
  4,425 tests; and
- each record's own review text supplies the local preserve/change decision.

No new tests or benchmarks are required because this batch makes no
implementation change. Focused prior slices remain the stronger evidence where
a record names a particular exact matrix, benchmark, or ownership regression.

## Falsification boundary

This is not a blanket closure of `E_Rust_Port-j76.4`, and classification is
routing rather than semantic proof. The `j76.4.1288` falsification demonstrates
that even affirmative design language can conceal an implementation mismatch;
focused source comparison and tests override the batch classification. The
classifier otherwise excludes every record that mentions missing or
unimplemented behavior, temporary or fallback ownership, provisional Rust
coverage, future port/integration work, or that lacks an affirmative decision.
Those records remain open for later source-unit and semantic reconciliation.

## Validation

- detailed migrated corpus: 1,327 unique contiguous records;
- qualifying decisions: 586;
- standard content hashes: 583/583;
- nonstandard full-text identities: three, included in the aggregate digest;
- final compatibility evidence checks: 3/3; and
- audit reference rerun: exact.

Reproduce locally:

```powershell
python experiments/2026-07-25-038-detailed-preservation-decisions/audit_preservation_decisions.py `
  --repo . `
  --expected experiments/2026-07-25-038-detailed-preservation-decisions/audit-reference.json
```
