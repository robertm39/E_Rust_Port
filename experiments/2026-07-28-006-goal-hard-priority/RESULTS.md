# Goal-hard-priority escalation results

- Contract: `0bad4c08f71d0fe1ff0e90c4f7f1780bba1dae6d0f152c41dfb7347c8ad42d4b`
- Prior contract: `2c8c13e468c19741c8fbc1ee8f56629c9d3d1519cc92da205a93781b20bfa42a`
- Umlaut binary SHA-256: `bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`
- Problems/families/runs: 23/6/276
- Telemetry: 203 valid, 0 invalid, 73 missing

## Strategy results

| Budget | Strategy | Reproducible solves | Median solved CPU (s) | Median solved generated/processed | Max schedule gap | Max preferred wait |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| larger | `global_aw` | 3 | 1.418314 | 0.428571 | 5 | 0 |
| larger | `goal_hard_priority` | 3 | 1.401837 | 0.428571 | 5 | 1 |
| larger | `goal_relevance_scalar` | 3 | 1.230356 | 0.428571 | 5 | 0 |
| short | `global_aw` | 2 | 0.712087 | 0.214286 | 5 | 0 |
| short | `goal_hard_priority` | 2 | 0.69771 | 0.214286 | 5 | 1 |
| short | `goal_relevance_scalar` | 3 | 1.299886 | 0.428571 | 5 | 0 |

## Coverage comparisons

### Larger Hard Vs Baseline

`goal_hard_priority` solved 3; `global_aw` solved 3.

- Left-only: none
- Right-only: none
- Common: ['ANA127^1', 'EEE001+1', 'SLH0044^1']

### Larger Hard Vs Scalar

`goal_hard_priority` solved 3; `goal_relevance_scalar` solved 3.

- Left-only: none
- Right-only: none
- Common: ['ANA127^1', 'EEE001+1', 'SLH0044^1']

### Short Hard Vs Baseline

`goal_hard_priority` solved 2; `global_aw` solved 2.

- Left-only: none
- Right-only: none
- Common: ['ANA127^1', 'EEE001+1']

### Short Hard Vs Scalar

`goal_hard_priority` solved 2; `goal_relevance_scalar` solved 3.

- Left-only: none
- Right-only: ['SLH0044^1']
- Common: ['ANA127^1', 'EEE001+1']

## Decision

- Goal hard priority: `reject`.
- Larger-budget net gain: 0.
- Paired hard/baseline CPU ratios: {'larger': 1.047415, 'short': 0.988121}.
- Short-budget hard-only solves retained at larger budget: none.
- Contradictory statuses: 0.
- Fairness bound violations: 0.
- Criterion: Advance at 20 seconds for a net held-out coverage gain of at least two, or identical coverage with a paired median CPU ratio at or below 0.8, with zero contradictory statuses and zero schedule-fairness bound violations.
