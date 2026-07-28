# Stronger redundancy results

- Selected strategy: `condensation`
- Umlaut binary SHA-256: `bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`
- Problems: 24 calibration, 24 validation, 20 test
- Runs: 752

## End-to-end results

| Phase | Budget | Strategy | Solves | By category | Median CPU (s) | Generated | High-water | Max RSS pages |
| --- | --- | --- | ---: | --- | ---: | ---: | ---: | ---: |
| calibration | calibration | `aggressive_forward_subsumption` | 3 | `{'EPS': 1, 'FEQ': 1, 'UEQ': 1}` | 3.431515 | 1342 | 44694 | 307184 |
| calibration | calibration | `baseline` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.702398 | 671.0 | 22352.0 | 216832.0 |
| calibration | calibration | `baseline_direct` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.953505 | 671.0 | 22352.0 | 211074.0 |
| calibration | calibration | `condensation` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.59305 | 671.0 | 22352.0 | 214916.0 |
| calibration | calibration | `condensation_full` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.752523 | 671.0 | 22352.0 | 214850.0 |
| calibration | calibration | `contextual_sr` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.6506 | 671.0 | 22352.0 | 217986.0 |
| calibration | calibration | `contextual_sr_full` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.834917 | 671.0 | 22352.0 | 214788.0 |
| calibration | calibration | `redundancy_bundle` | 4 | `{'EPS': 1, 'FEQ': 1, 'UEQ': 2}` | 2.231145 | 22717.0 | 42309.0 | 184538.0 |
| calibration | calibration | `strong_demodulation` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.679927 | 671.0 | 22352.0 | 207890.0 |
| calibration | calibration | `strong_unit_subsumption` | 2 | `{'EPS': 1, 'FEQ': 1}` | 1.609009 | 671.0 | 22352.0 | 217986.0 |
| validation | validation | `aggressive_forward_subsumption` | 5 | `{'EPS': 2, 'FNE': 1, 'UEQ': 2}` | 0.144304 | 19001.0 | 2321.0 | 40948.0 |
| validation | validation | `baseline` | 6 | `{'EPS': 2, 'FNE': 2, 'UEQ': 2}` | 0.723426 | 42668.0 | 5417.5 | 50570.0 |
| validation | validation | `condensation` | 6 | `{'EPS': 2, 'FNE': 2, 'UEQ': 2}` | 0.733754 | 42668.0 | 5417.5 | 49708.0 |
| validation | validation | `redundancy_bundle` | 5 | `{'EPS': 2, 'FNE': 1, 'UEQ': 2}` | 0.154078 | 19001.0 | 2195.0 | 40948.0 |
| test | larger | `baseline` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.075525 | 7603.0 | 6961.0 | 53950.0 |
| test | larger | `baseline_direct` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.073102 | 7603.0 | 6961.0 | 53028.0 |
| test | larger | `condensation` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.077246 | 7650.0 | 7005.0 | 54872.0 |
| test | larger | `selected_direct` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.075232 | 7650.0 | 7005.0 | 53950.0 |
| test | short | `baseline` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.075714 | 7603.0 | 6961.0 | 54872.0 |
| test | short | `baseline_direct` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.0742 | 7603.0 | 6961.0 | 53028.0 |
| test | short | `condensation` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.076214 | 7650.0 | 7005.0 | 53950.0 |
| test | short | `selected_direct` | 3 | `{'FEQ': 1, 'UEQ': 2}` | 0.07523 | 7650.0 | 7005.0 | 53950.0 |

## Held-out selected versus baseline

### larger

Selected solved 3; baseline solved 3.

- Selected-only: none
- Baseline-only: none
- All-run paired CPU ratio: 1.000137
- All-run paired generated ratio: 0.994428
- All-run paired final-clause ratio: 0.999257
- All-run paired high-water ratio: 0.999257
- All-run paired max-RSS-pages ratio: 0.998779

### short

Selected solved 3; baseline solved 3.

- Selected-only: none
- Baseline-only: none
- All-run paired CPU ratio: 1.000102
- All-run paired generated ratio: 1.0
- All-run paired final-clause ratio: 1.0
- All-run paired high-water ratio: 1.0
- All-run paired max-RSS-pages ratio: 0.990319

## Slow-reference audit

- baseline: 12 terminal pairs, 0 polarity disagreements.
- selected: 12 terminal pairs, 0 polarity disagreements.

## Independent proof validation

ProofCheck verified 12 of 12 reproducible proof claims.

## Decision

- Result: `retain_existing_redundancy_defaults`.
- Observed selected/baseline behavior differences: 34 coordinates.
- Contradictory statuses: 0.
