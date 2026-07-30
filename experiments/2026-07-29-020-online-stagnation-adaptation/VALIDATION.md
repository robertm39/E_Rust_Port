# Validation freeze

The independent validation phase completed all 80 policy coordinates and passed
the status, PCL proof, timeout, configured-budget, raw-hash, contract, and
adaptive-branch correctness gates.

- Contract ID:
  `603dccf2e5b15ee8b3cba907be4ca04cbb10ff109403cfe27b113a072a144468`
- Analysis report ID:
  `2227cf02fdd5c10bca2ad84e24e3200893719880eb5ff849ddd6a36614bc57a2`
- Analysis file SHA-256:
  `91ac9e14386f69d8eae64bc22b2625c7de7f5d54340845566ca2e0772072a158`
- Frozen selection ID:
  `a53a56b93b549dd8258801e91535590e65a5e6e4394df16fe145bb0648c1d15c`
- Selected threshold: `64`

Every policy reproducibly solved only `PUZ008-2`; there were no one-repeat,
adaptive-only, or comparator-only solves. The adaptive-to-static-goal median
CPU ratio was `0.760391325`, but it covers only the two repetitions of that one
common solve. The preregistered efficiency gate requires at least four common
solved repetition coordinates, so validation makes no efficiency claim.

Adaptive decisions were stable across both repetitions for every problem:
eight coordinates restarted global, six took the deterministic goal fallback,
and two solved during the probe. Maximum measured decision CPU and wall
overhead were 24.687 and 24.267 microseconds, respectively, well under the
10-millisecond limit.

Fifty-eight phase telemetry files across all policies were absent after kernel
hard stops. Their raw stdout/stderr and absence remain hash-validated. The
missing telemetry neither changed a branch from the frozen fallback rule nor
invalidated a proof/status result, but it limits aggregate CPU evidence and is
carried into the final sufficiency decision.

The validation report outcome is `ready_for_test`. It authorizes the frozen
test phase but makes no production recommendation.

The ignored raw files are retained at:

```text
.artifacts/experiments/2026-07-29-020-online-stagnation-adaptation/
```

The authoritative remote raw root is
`/opt/e-rust-port/online-adaptation-020/validation-v1`.
