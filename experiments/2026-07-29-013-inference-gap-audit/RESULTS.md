# Inference-gap audit results

- Search contract: `870f13dc65aac6b14973c9a7c85dfbd39d3211761402b8861a4bda855cd7646f`
- Binary SHA-256: `db84f7d4a12927adb730a46930b065f2e919156a4b77747a5d40b79bd2a78ec6`
- Matrix report: `8837eb8057aac25b3c6052d5e68f2079abc17597b035cea172294b2af6659a57`
- Search runs: 160
- Independent proof claims: 6/6 verified

| Budget | Baseline solves | Local-rw solves | Candidate-only | Baseline-only | Common CPU | Common generated | Common high-water | All-run generated | Max RSS | Effect coordinates |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| larger | 3 | 3 | 0 | 0 | 1.078112 | 1.0 | 1.0 | 0.966464 | 1.046151 | 33 |
| short | 3 | 3 | 0 | 0 | 1.078169 | 1.0 | 1.0 | 0.95548 | 0.910997 | 35 |

Decision: `retain_local_rewriting_as_default_off`.

No checked proof emitted a `local_rw` step.
