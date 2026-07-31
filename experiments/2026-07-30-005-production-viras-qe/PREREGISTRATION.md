# Production base VIRAS QE preregistration

Bead: `E_Rust_Port-9jt.5.11`

## Question

Does the clean-room Rust port reproduce the frozen base VIRAS and typed
adapter boundaries, fail closed under every exposed limit, publish
replay-validated transformations, and provide useful standalone decisions on
family-disjoint typed arithmetic without changing Umlaut's default schedules
or package?

## Frozen implementation boundary

The evaluated implementation is the already written opt-in `viras-qe` feature
before this experiment is executed. No kernel rule, importer case, resource
default, generated-family parameter, or acceptance threshold may be tuned
after observing the report. An explained implementation defect requires a
new report and explicit disclosure.

The implementation may use only the tracked clean-room `viras_docs/` packet,
the frozen experiments 023 and 004, Umlaut's existing typed AST, and the
audited pure-Rust `num` graph. It must not inspect, import, build, execute, or
derive cases from the unlicensed VIRAS source.

## Held-out surfaces

### Untouched CASC documents

Scan all 100 files in `problems/casc_2025/TFI`. These problems were not used to
design the one-formula document gate or the generated arithmetic families.
Report accepted-success, accepted-Unknown, and every stable rejection code.
For each successful closed result with a TPTP status comment, exact evaluation
of the canonical result must agree with that expected truth value.

Low coverage is reportable and does not invalidate soundness. It must not be
hidden by extracting formulas, expanding includes, or silently discarding
type declarations.

### Analytic TFA families

Generate 20 cases from each of six new deterministic families:

1. existential integer intervals;
2. existential real floor bands;
3. scaled-floor interval intersections;
4. universal real gap coverage;
5. existential Boolean point alternatives; and
6. nested universal/existential affine constraints.

Use seed `0x51A52026`. Every expected truth value is computed directly from
integer or `fractions.Fraction` interval algebra in the controller, not from
the Rust result or an SMT solver. These families are disjoint from experiment
004's bounded random conjunction generator and collectively cover integers,
reals, floor, scaling, Boolean structure, universals, and nested quantifiers.

## Frozen gates

The production bead passes only if:

1. focused Rust tests reproduce all frozen profile, grid, candidate, V1/V2/V3,
   motivating, mutation, 1,000-decision, and experiment-023 importer records;
2. all 120 analytic documents import and eliminate successfully;
3. exact independent evaluation of every quantifier-free result agrees with
   the analytic expected value, with at least 20 true and 20 false cases;
4. every successful record has `replay_validated=true`, no remaining
   quantifier or free variable, a result formula, and a TFF re-embedding;
5. two invocations are byte-identical for two stratified cases per family;
6. result-flip, TFF-flip, replay-flag, and candidate/derivation corruptions are
   rejected by the independent record checker or byte-for-byte replay;
7. zero limits for steps, candidates, grids, grid points, DNF branches,
   formula nodes, and rational bits each return JSON `Unknown(ResourceLimit)`
   with no result formula;
8. coverage, rejection taxonomy, median/p95/max process latency, imported and
   result node counts, p95/max growth, and candidate/grid maxima are reported;
9. default `umlaut` is run under `--auto --cpu-limit=1 --memory-limit=2048` on
   the 120 analytic cases and exact correct-solve overlap/QE-only/default-only
   complementarity is reported;
10. Linux all-feature tests/clippy/fmt, Windows-GNU compile-only, default
    package audit, and the repository's comprehensive runner pass; and
11. `src/heuristics/schedule.vars` remains at SHA-256
    `491145ab45477620ed02ed8cd789d6b5e3e6e0d38f413fdbc62163e09a9cb068`,
    the default feature list remains empty, and the default package omits
    `umlaut-viras-qe` and all optional crate code.

## Metrics and interpretation

Latency is whole-process elapsed time, so it includes typed parsing and
startup. Formula growth is canonical AST node count after elimination divided
by imported node count; report the distribution without imposing an efficacy
threshold. A correct standalone QE decision counts as a solve even though it
is deliberately not integrated into `umlaut`. A default-prover outcome counts
only when its SZS status matches the analytic expected truth.

## Stop rules

Stop without closing the bead if any semantic disagreement remains, a
successful record fails replay, corruption is accepted, a resource limit
publishes a formula, the optional graph leaks into the default package, or a
comprehensive compatibility gate fails. Low real-corpus coverage or no
default-prover complementarity is a documented utility limitation, not a
soundness exception and not permission to enable automatic scheduling.
