# Inference-processing and SAT-check option matrix

## Status

Completed for Bead `E_Rust_Port-j76.2.86`. The focused executable surface is
19/19 byte-exact against the pinned C reference. One production reporting bug
was found and fixed; the vendored C source remained unchanged.

## Question

Do the migrated inference-processing and SAT-check options still expose a
drop-in compatibility gap, and which residual SAT and higher-order work belongs
to narrower Beads?

## Method

[`compare_surfaces.py`](compare_surfaces.py) runs the Windows release binary and
the isolated WSL C executable from commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`. Exit status, stdout, and stderr are
compared without normalization; only fixture paths are translated to WSL.

The matrix exercises:

- combined inference-processing controls, including destructive equality
  resolution, contextual simplify-reflect, demodulation, strong rewriting,
  strong forward subsumption, SOS input types, and lambda lifting;
- disabled or independently enabled inference-processing controls;
- SATCheck grounding, processed/generated/term-bank triggers, the decision
  limit, constant normalization, unprocessed-clause normalization, and
  optional defaults; and
- invalid forward-demodulation, lambda, grounding, interval, and decision-limit
  arguments.

## Results

All 19 cases are byte-exact:

| Case group | Cases | Result |
| --- | ---: | :---: |
| inference-processing behavior and option arity | 5 | exact |
| SATCheck grounding, triggers, normalization, and defaults | 8 | exact |
| invalid inference/SATCheck arguments | 6 | exact |

The compact hashes and complete mismatch payloads, if a future run regresses,
are retained in [`results-summary.json`](results-summary.json).

## Corrected SATCheck return classification

The initial matrix was 18/19 exact. With `--satcheck-normalize-unproc`, both
implementations found the same empty clause during the preliminary
`ForwardContractSetReweight` pass, but Rust printed
`% SatCheck found unsatisfiable ground set` while C did not.

In C, that comment belongs to `SatClauseSetCheckUnsat`: it reports a SAT solver
UNSAT result. An empty clause found by SATCheck's normalization pass returns
from `SATCheck` before the solver is called, so it neither prints the solver
comment nor increments solver-result statistics. Rust had represented both
paths with one `SaturateReturnReason::SatCheck` value, and the executable used
that value as permission to print the comment.

Rust now carries an internal `solver_reported` distinction out of the SATCheck
gate and maps preprocessing refutations to
`SaturateReturnReason::SatCheckPreprocessing`. Both paths still record the empty
clause as an extraction root, but only an actual solver UNSAT result uses
`SaturateReturnReason::SatCheck` and emits the C comment.

## Scope decision

The broader LFHO paramodulation/solution work migrated into this umbrella was
already completed and closed under `E_Rust_Port-j76.1.5`, with higher-order
release-oracle evidence. Default PicoSAT packaging, exact solver-core
extraction, and solver-specific decision behavior remain under
`E_Rust_Port-j76.3.229`. Optional SAT defaults and the historical diagnostic
typo remain under `E_Rust_Port-j76.3.227` and `E_Rust_Port-j76.3.228`.

Those narrower items do not represent missing integration of the option surface
audited here.

## Permanent Rust coverage

New regressions pin the saturation-level distinction between normalization and
solver refutations, including extraction-root ownership, SATCheck statistics,
and solver generation. An executable regression pins the absence of the solver
comment when preprocessing finds the empty clause. Existing tests cover option
parsing/config propagation, SATCheck scheduling, solver reset, grounding,
normalization, core extraction, and runtime PicoSAT dispatch.

## Validation

- focused C/Rust matrix: 19/19 exact;
- focused SATCheck Rust tests: passed;
- full all-target, all-feature Rust suite: 4,259 library tests plus every binary
  and integration target passed under Cargo's default parallel runner;
- strict pedantic Clippy: passed;
- release `eprover` build: passed; and
- formatting, experiment-script compilation, and all four C-source
  documentation integrity gates: passed.
