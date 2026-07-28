# Unit-equality completion results

- Calibration contract: `4b743ae625756002aaf531618edacb5057e78aa565f3444f0124064c37c35446`
- Validation contract: `647075259b7d2f8eca0696cd2b7215d718d18d0faf51239ba82400aa8ac42d1e`
- Test contract: `cae51c0877924d0a360ab03c03e97ca312beff6e2abb1a514e4c5326ceef8146`
- Umlaut binary SHA-256: `bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`
- Problems: 28 calibration, 20 validation, 20 held-out test
- Runs: 692
- Validation-selected completion strategy: `completion_ac_units`

## Strategy results

| Phase | Budget | Strategy | Solves | Median solved CPU (s) | Median paramodulations | Median rewrite steps | Median high-water clauses | Median max RSS pages |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| calibration | calibration | `auto_general` | 10 | 0.165017 | 22714.5 | 21758.5 | 12414.5 | 63624.0 |
| calibration | calibration | `completion_ac_units` | 7 | 0.273205 | 31432 | 79575 | 15514 | 79940 |
| calibration | calibration | `completion_initial` | 7 | 0.230925 | 16889 | 52039 | 11951 | 62108 |
| calibration | calibration | `completion_lpo` | 2 | 0.622775 | 62106.0 | 66198.5 | 60420.5 | 196746.0 |
| calibration | calibration | `completion_presat` | 7 | 0.323306 | 30665 | 79575 | 14873 | 80052 |
| calibration | calibration | `completion_queue` | 7 | 0.424565 | 28209 | 61852 | 30469 | 71212 |
| calibration | calibration | `completion_simul` | 7 | 0.29691 | 26313 | 69091 | 11936 | 72028 |
| calibration | calibration | `completion_strong_rw` | 7 | 0.292863 | 30965 | 79575 | 15826 | 80052 |
| calibration | calibration | `manual_general` | 6 | 0.46951 | 32396.0 | 69827.0 | 33082.5 | 80880.0 |
| validation | validation | `auto_general` | 8 | 0.544615 | 48871.0 | 86347.0 | 14731.5 | 72864.0 |
| validation | validation | `completion_ac_units` | 7 | 0.146126 | 19762.0 | 25316.0 | 6256.0 | 38724.0 |
| validation | validation | `completion_initial` | 7 | 0.16883 | 22064.0 | 29155.0 | 4543.0 | 38724.0 |
| validation | validation | `completion_strong_rw` | 7 | 0.147317 | 19422.0 | 25316.0 | 6256.0 | 38724.0 |
| validation | validation | `manual_general` | 7 | 0.145539 | 19001.0 | 24870.0 | 3149.0 | 38724.0 |
| test | larger | `auto_general` | 10 | 0.653012 | 110903.5 | 180842.5 | 49682.0 | 90680.0 |
| test | larger | `completion_ac_units` | 5 | 0.255563 | 25608.0 | 54396.0 | 14731.0 | 41050.0 |
| test | larger | `manual_general` | 4 | 2.573975 | 193191.0 | 514626.0 | 98888.5 | 191992.0 |
| test | short | `auto_general` | 7 | 0.510731 | 78117.0 | 158413.0 | 35737.0 | 75880.0 |
| test | short | `completion_ac_units` | 5 | 0.252878 | 25608.0 | 54396.0 | 14731.0 | 41116.0 |
| test | short | `manual_general` | 3 | 0.296799 | 18148.0 | 39954.0 | 14397.0 | 37140.0 |

## Held-out comparisons

### larger: chosen vs auto

`completion_ac_units` solved 5; `auto_general` solved 10; portfolio union 10.

- Left-only: none
- Right-only: ['MVA008-1', 'MVA009-1', 'MVA011-1', 'REL012-1', 'REL031-1']
- Paired median CPU ratio: 0.514474
- Paired median generated-clause ratio: 0.327816
- Paired median paramodulation ratio: 0.327816
- Paired median rewrite-step ratio: 0.347931
- Paired median high-water ratio: 0.412206

### larger: chosen vs manual

`completion_ac_units` solved 5; `manual_general` solved 4; portfolio union 6.

- Left-only: ['REL027-1', 'REL045-2']
- Right-only: ['REL012-1']
- Paired median CPU ratio: 0.161429
- Paired median generated-clause ratio: 0.225259
- Paired median paramodulation ratio: 0.225259
- Paired median rewrite-step ratio: 0.211994
- Paired median high-water ratio: 0.204348

### short: chosen vs auto

`completion_ac_units` solved 5; `auto_general` solved 7; portfolio union 7.

- Left-only: none
- Right-only: ['MVA011-1', 'REL031-1']
- Paired median CPU ratio: 0.516366
- Paired median generated-clause ratio: 0.327816
- Paired median paramodulation ratio: 0.327816
- Paired median rewrite-step ratio: 0.347931
- Paired median high-water ratio: 0.412206

### short: chosen vs manual

`completion_ac_units` solved 5; `manual_general` solved 3; portfolio union 6.

- Left-only: ['REL026-1', 'REL027-1', 'REL045-2']
- Right-only: ['REL012-1']
- Paired median CPU ratio: 1.064673
- Paired median generated-clause ratio: 1.052579
- Paired median paramodulation ratio: 1.052579
- Paired median rewrite-step ratio: 1.107091
- Paired median high-water ratio: 1.193137

## Independent proof validation

ProofCheck verified 19 of 19 reproducible larger-budget strategy/problem claims.

## Decision

- Result: `reject_separate_completion_engine`.
- Reason: the selected completion configuration did not produce two held-out unique solves or a non-inferior 10% paired CPU gain.
- Contradictory statuses: 0.
