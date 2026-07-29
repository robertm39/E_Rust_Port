# Empirical experiment contracts

Umlaut experiments should make it cheap to reject weak ideas without turning
every investigation into a statistics project. The version-1 result contract
records only the information needed to reproduce a comparison, keep
correctness separate from speed, understand observed variation, and apply a
declared decision rule.

The reusable files are:

- [`experiment-result.schema.json`](../tools/experiment_contract/experiment-result.schema.json),
  the machine-readable shape;
- [`template.json`](../tools/experiment_contract/template.json), a valid
  copy-and-edit starting point; and
- [`validate.py`](../tools/experiment_contract/validate.py), the
  standard-library validator and artifact-integrity checker.

The schema complements the repository's `manage-experiments` workflow. It does
not replace `PREREGISTRATION.md`, `FINDINGS.md`, raw run records, or a
domain-specific analyzer. Put experiment-specific controllers and result
contracts in the dated experiment directory, and keep the reusable validator
under `tools/`.

## Required decisions

Every result has these distinct sections:

1. `experiment` and `treatments` state the falsifiable hypothesis and the one
   intended difference between baseline and candidate.
2. `reproduction` pins the source revision, platform, seed policy, resource
   limits, commands, and integrity-addressed artifacts.
3. `correctness` records status/polarity pairing and independent proof or model
   checks. It is evaluated without reference to timing.
4. `performance` states whether timing is valid, the paired unit, primary
   direction, effect, repeat variation, and any secondary guard metrics.
5. `coverage` exposes common, unique, and lost reproducible solves.
6. `decision` is exactly `continue`, `stop`, or `uncertain`, with the frozen
   rule, reasons, and production effect.

`continue` is structurally forbidden unless correctness passes. A coverage gap
may still permit correctness to pass when the preregistered gate requires a
verified candidate witness and rejects any independently bad result, but the
gap must remain explicit. Invalid or missing performance must not be rewritten
as a speed result; use `uncertain` unless a correctness failure or an
independent non-timing rule already requires `stop`.

## Pairing and noise

Pair baseline and candidate by problem, resource budget, repetition, and every
other search-affecting input. Aggregate ratios only over coordinates for which
both values exist and have the declared interpretation. Timeout-limited CPU
totals are not proof of a speedup.

For ordinary repeated timing, version 1 uses the within-coordinate relative
range:

```text
(maximum repetition value - minimum repetition value)
------------------------------------------------------
             median repetition value
```

Record the median and maximum of that quantity across baseline coordinates,
candidate coordinates, and the paired candidate/baseline ratios. This compact
summary answers whether the observed effect is large compared with repetition
movement without pretending that two repeats establish a confidence interval.
Preserve the raw per-run values so later work can use a stronger method.

Two or more repetitions are the normal minimum for time or memory. A
deterministic operation counter may use exact replay instead; name that method
and report zero variation only after verifying the replay is exact. Seeds are
not mandatory when the program has no RNG, but the record must say so and pin
the deterministic selection policy.

## Workflow

Before looking at candidate results:

1. create the next dated `experiments/YYYY-MM-DD-NNN-slug/` directory;
2. freeze the hypothesis, treatments, held-out selection, budgets, repeats,
   correctness gates, and decision thresholds;
3. copy the template and fill the reproduction fields; and
4. run prover experiments and Rust builds only through
   [`linode-runner.ps1`](../linode-runner.ps1).

After execution:

1. run independent correctness validation first;
2. mark performance `invalid` if its own integrity, resume, telemetry, or
   pairing checks fail;
3. calculate the paired effect and variation without changing the frozen
   decision rule;
4. record unique and lost solves separately from aggregate timing;
5. retain raw outputs under `.artifacts/experiments/<experiment-id>/` and
   record their size and SHA-256; and
6. validate the compact record from the repository root:

```text
python tools/experiment_contract/validate.py \
  --verify-artifacts \
  experiments/YYYY-MM-DD-NNN-slug/result.json
```

The artifact check deliberately fails when ignored raw evidence is absent.
Run structural validation without `--verify-artifacts` when reviewing a clone
that has only the compact tracked record.

## Trial evidence

The version-1 contract was trialed on one default-off preprocessing toggle and
one proof-preserving cache performance comparison in
[`experiments/2026-07-29-016-experiment-contract-trials/`](../experiments/2026-07-29-016-experiment-contract-trials/).
The trial verifier recomputes coverage, status pairing, paired CPU ratios, and
repeat variation directly from the two preserved raw archives, then checks the
source analyzers' proof and decision reports.
