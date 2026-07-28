# Unit-equality completion specialist

Bead: `E_Rust_Port-9jt.6.4`

## Question

Can completion-oriented use of Umlaut's existing superposition engine improve
the CASC UEQ division enough to justify a dedicated specialist, or does tuned
general saturation already dominate the available completion controls?

## Capability audit

Umlaut has no separate unfailing-completion loop, critical-pair store, or
ground-joinability service. It does already provide the core operations a
configuration-first study needs:

- ordered and indexed paramodulation with ordinary, simultaneous, and
  supersimultaneous variants;
- forward and backward demodulation, strong rewrite instantiation, and
  presaturation interreduction;
- KBO6 and LPO4 term orderings with configurable precedence generation;
- orientability-aware, goal-aware, and age/fairness given-clause queues;
- AC redundancy modes; and
- proof objects plus aggregate pair-generation, rewrite, search-state, CPU,
  and resident-memory telemetry.

Equality factoring is irrelevant on the positive unit Horn axioms in the
target class. Literal selection also has no branching choice on unit clauses.
The completion candidates therefore disable factoring and use
`NoSelection`, while preserving ordinary ordered superposition completeness
for the unit-equational class.

## Preregistered experiment

The pinned CASC-30 manifest contains 300 UEQ problems, all independently
classified as unsatisfiable. Its family split is unusually useful here:

- training contains 200 problems from nine families;
- validation contains 52 problems from five different families; and
- test contains 48 problems from four further families.

No source family crosses a split. The harness selects 28/20/20 problems with
equalized family quotas and evenly spaced within-family difficulty order.

The stages are:

1. Calibrate automatic general saturation, a fixed manual general baseline,
   and seven incremental completion configurations for one repetition at a
   four-second soft CPU budget.
2. Rank only the completion configurations by solve count, median solved CPU,
   median generated clauses, and stable name. Carry the top three into
   two-repetition, eight-second validation beside both general baselines.
3. Select one completion configuration by the same validation-only rule.
4. Compare that frozen configuration with both general baselines on the
   untouched test families, with two repetitions at five- and twenty-second
   soft CPU budgets.

The seven specialist candidates isolate an orientability/goal queue, initial
interreduction, simultaneous paramodulation, strong rewrite instantiation,
LPO4/inverse-frequency precedence, unit-preserving AC handling, and
initial-equations-first processing. Exact argument vectors are contract-bound
in `run.py`.

Every test run emits a TSTP proof object. Before the final decision,
ProofCheck 1.0 must return `VerifiedGood` through Umlaut's positive-only
validation controller for one proof from every reproducibly solved
strategy/problem pair at the larger budget. Matching statuses are never used
as proof verification.

## Decision rule

Advance the selected completion configuration only if:

- it adds at least two reproducible held-out solves beyond automatic general
  saturation; or
- it loses no automatic-general solve, has a paired median CPU ratio at most
  `0.90`, and has a paired median search-state high-water ratio at most
  `1.05`.

Both paths additionally require zero contradictory SZS statuses and
independent verification of every larger-budget proof claim used by the
comparison. Otherwise reject a separate completion engine at this stage.

Pair generation is reported as generated paramodulants and paramodulations per
processed clause. Normalization work is reported as rewrite steps and rewrite
steps per processed clause. Total CPU, generated clauses, high-water clause
count, and maximum resident pages bound their cost.

## Status

Completed. The registered decision is
`reject_separate_completion_engine`.

## Results

All 692 contract-bound runs completed:

- calibration: 252 runs over 28 training problems;
- validation: 200 runs over 20 family-disjoint validation problems; and
- test: 240 runs over 20 untouched test problems.

The exact-resume check recovered 252/252 calibration, 200/200 validation, and
240/240 test results without executing another solver run.

Automatic general saturation solved 10 calibration problems. Six completion
variants tied at seven solves, while LPO4 solved two. The registered ranking
advanced `completion_initial`, `completion_ac_units`, and
`completion_strong_rw`. On validation, all three solved seven problems.
`completion_ac_units` won the validation-only tie-break with a 0.146126-second
median solved CPU time, narrowly ahead of `completion_strong_rw` at 0.147317
seconds and `completion_initial` at 0.168830 seconds.

At the larger held-out budget, the frozen `completion_ac_units` configuration
solved five problems and automatic general saturation solved ten. Every
completion solve was also an automatic solve. Automatic general saturation
additionally solved `MVA008-1`, `MVA009-1`, `MVA011-1`, `REL012-1`, and
`REL031-1`. On their five common solves, the completion configuration used:

- 0.514474 times the paired CPU;
- 0.327816 times the generated clauses and paramodulations;
- 0.347931 times the rewrite steps; and
- 0.412206 times the search-state high-water clauses.

The completion configuration was therefore substantially cheaper where it
worked, but its coverage was a strict subset of automatic general saturation.
It neither added two unique solves nor preserved coverage, so the registered
efficiency exception does not apply. The five-second comparison reached the
same conclusion: five completion solves versus seven automatic solves, with
no completion-only problem.

Against the fixed manual baseline at the larger budget, completion solved five
and manual saturation solved four. Completion uniquely solved `REL027-1` and
`REL045-2`, while manual saturation uniquely solved `REL012-1`. This is useful
configuration evidence, but it does not overcome the automatic baseline.

## Proof validation

ProofCheck 1.0 self-certified all 117 bundled tests. Its strict source-leaf
check compares variable spelling rather than alpha-equivalence: for example,
it rejects a proof leaf using `X1,X2` against the cited source clause using
`X,Y`, even when every token is otherwise identical. The experiment therefore
uses a narrow checker adapter:

1. parse every file-cited CNF leaf and its original source;
2. require token-for-token alpha-equivalence under a bijective first-occurrence
   variable renaming;
3. create a per-proof controller problem containing the proof's variable
   spelling; and
4. change only each leaf's `file()` target.

The adapter does not change a proof formula, role, inference name, status,
parent, or conclusion. Every leaf audit and original/prepared/controller hash
is recorded per case. ProofCheck returned `VerifiedGood` for all 19
reproducibly solved larger-budget strategy/problem claims. The hash-bound
validation report ID is
`566953ea7c44f85c621c7a6281c7344b0bda9e14995bfc97aa51d0cb0682b8b6`.
No run reported a contradictory satisfiable status.

## Reproducibility

- calibration contract:
  `4b743ae625756002aaf531618edacb5057e78aa565f3444f0124064c37c35446`;
- validation contract:
  `647075259b7d2f8eca0696cd2b7215d718d18d0faf51239ba82400aa8ac42d1e`;
- test contract:
  `cae51c0877924d0a360ab03c03e97ca312beff6e2abb1a514e4c5326ceef8146`;
- calibration selection:
  `cda4e9d636e715f634a5971acf1e1cf3e04e757360debc2f43ac69ff57db7c04`;
- final selection:
  `65b8ce73ca80f3eea9edb9679b075768bad90f487f5067f924f93d8454299e1d`;
- Umlaut release binary:
  `bfa6905a29c80c50420279ded641d46f0517de03ea85a9f4c28140a0c9065ea0`;
- raw run and proof-evidence archive: 21,553,704 bytes, SHA-256
  `f6973fa0e1cedea27ee6e2dec10bc5c609114e47676a14edd1bba494e8ff64c0`,
  stored at
  `.artifacts/unit-equality-completion/ueq-completion-raw.tar.gz`; and
- full comparison tables and proof metadata:
  `RESULTS.md` and `results-summary.json`.

## Decision

Do not add a separate unit-equality completion loop or route UEQ problems to
`completion_ac_units`. The configuration is a useful low-cost fallback on its
five-solve subset, but the automatic strategy already solves that entire
subset and five additional held-out problems. A future UEQ study should focus
on predicting when the cheaper configuration is safe or on recovering the
missing MVA/REL coverage, rather than implementing a new completion engine
from the present configuration.
