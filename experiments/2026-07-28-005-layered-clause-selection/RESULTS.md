# Layered clause-selection results

- Contract: `2c8c13e468c19741c8fbc1ee8f56629c9d3d1519cc92da205a93781b20bfa42a`
- Umlaut binary SHA-256: `bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`
- Problems: 44
- Runs: 704
- Telemetry: 546 valid, 0 invalid, 158 missing
- Validation-selected layered strategy: `goal_layered_4_1`

## Strategy results

| Split | Strategy | Reproducible solves | Median solved CPU (s) | Median solved generated/processed | Max schedule gap | Max preferred wait | Bad deleted |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| validation | `global_aw` | 7 | 1.074159 | 5.209524 | 5 | 0 | 0 |
| validation | `global_static_prune` | 4 | 0.124394 | 10.608085 | 5 | 0 | 6085669 |
| validation | `goal_hard_priority` | 7 | 0.358437 | 6.021212 | 5 | 1 | 0 |
| validation | `goal_layered_1_4` | 7 | 1.055072 | 5.143292 | 5 | 5 | 0 |
| validation | `goal_layered_4_1` | 7 | 0.784381 | 5.872282 | 5 | 2 | 0 |
| validation | `goal_relevance_scalar` | 7 | 0.18297 | 2.697849 | 5 | 0 | 0 |
| validation | `horn_layered_4_1` | 7 | 1.058242 | 5.216528 | 5 | 2 | 0 |
| validation | `unit_layered_4_1` | 7 | 1.193833 | 8.856208 | 5 | 2 | 0 |
| test | `global_aw` | 0 | None | None | 5 | 0 | 0 |
| test | `global_static_prune` | 0 | None | None | 5 | 0 | 5715333 |
| test | `goal_hard_priority` | 3 | 4.297693 | 17.797255 | 5 | 1 | 0 |
| test | `goal_layered_1_4` | 0 | None | None | 5 | 5 | 0 |
| test | `goal_layered_4_1` | 1 | 0.011146 | 2.168 | 5 | 2 | 0 |
| test | `goal_relevance_scalar` | 0 | None | None | 5 | 0 | 0 |
| test | `horn_layered_4_1` | 0 | None | None | 5 | 2 | 0 |
| test | `unit_layered_4_1` | 0 | None | None | 5 | 2 | 0 |

## Coverage comparisons

### Validation Chosen Vs Baseline

`goal_layered_4_1` solved 7; `global_aw` solved 7; common 7.

- Left-only: none
- Right-only: none

### Test Chosen Vs Baseline

`goal_layered_4_1` solved 1; `global_aw` solved 0; common 0.

- Left-only: ['NUN060+1']
- Right-only: none

### Test Chosen Vs Scalar

`goal_layered_4_1` solved 1; `goal_relevance_scalar` solved 0; common 0.

- Left-only: ['NUN060+1']
- Right-only: none

### Test Hard Priority Vs Baseline

`goal_hard_priority` solved 3; `global_aw` solved 0; common 0.

- Left-only: ['NUN060+1', 'NUN085+1', 'SEU025+1']
- Right-only: none

### Test Static Prune Vs Baseline

`global_static_prune` solved 0; `global_aw` solved 0; common 0.

- Left-only: none
- Right-only: none

## Decision

- Layered selection: `reject_current_candidates`.
- Criterion: Advance only for at least two reproducible held-out unique solves, zero contradictory statuses, and zero schedule-fairness bound violations.
- Limited Resource Strategy: `reject_direct_port`.
- Rationale: Vampire LRS is an Otter-loop time-reachability policy. The measured static delete-bad control is only a falsification proxy for whether non-redundant passive pruning helps Umlaut's DISCOUNT loop; it is not labeled as LRS.
- Status mismatches: 0.
