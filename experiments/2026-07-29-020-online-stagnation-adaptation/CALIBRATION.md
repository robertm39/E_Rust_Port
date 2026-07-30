# Calibration freeze

The frozen calibration matrix completed all 80 primitive coordinates on the
Ubuntu runner with no bad status, proof-without-PCL, external-timeout,
configured-budget, or contract failure.

- Contract ID:
  `70bba21867ab8615333ea75b3efc9eb52dc5924d9d231a2344b914d531975bb6`
- Analysis report ID:
  `8b35b51407c8d122b37c142a8e7f3fe5a1e28b21b88373a834648e2c9e70946c`
- Analysis file SHA-256:
  `09ef852a4b0db5d8f4b5127fc8e94a3594bc24f0644144cb8a43dc1e4405ccfb`
- Selection ID:
  `a53a56b93b549dd8258801e91535590e65a5e6e4394df16fe145bb0648c1d15c`
- Selection file SHA-256:
  `0981e3f843a0754934675af03998622c0d3d41f0581ff61c44a2f8125ff39681`
- Selected clause-growth threshold: `64`

All five threshold candidates reproduced the same one calibration solve,
`LAT260-2`, with no loss versus the static global restart and no win versus the
static goal portfolio. The preregistered conservative tie break therefore
selected the highest threshold.

At the selected threshold, the 16 repetition coordinates made six valid
low-growth global decisions and ten deterministic goal-fallback decisions.
The valid clause-growth ratios were between 1.5099 and 1.5962. Four probe
coordinates lacked telemetry after a kernel hard stop; six more reported fewer
than the frozen 64 processed clauses. Both cases take the preregistered goal
fallback. Branches agreed across both repetitions for every problem.

Across all five primitive arms, 48 phase telemetry files were missing after
kernel hard stops. Those files are not synthesized and CPU aggregation excludes
their coordinates. The raw stdout/stderr and absence itself remain
hash-validated evidence. This limitation was visible before validation and does
not amend the policy, threshold, resource limits, corpus, or decision rule.

The ignored raw files are retained at:

```text
.artifacts/experiments/2026-07-29-020-online-stagnation-adaptation/
```

The authoritative remote raw root is
`/opt/e-rust-port/online-adaptation-020/calibration-v1`.
