# Higher-order inference gap audit

Bead: `E_Rust_Port-9jt.6.2`

## Question and preregistration

Which syntax, preprocessing, unification, calculus, and scheduling gaps explain
Umlaut's remaining CASC-30 THF failures, and does closing one scoped,
independently checkable gap improve held-out THF without slowing FOF?

This section is preregistered before corpus-run results are inspected.

## Capability audit

The audit compares Umlaut's implemented higher-order path with the bundled E
and pinned Vampire sources. It records, at minimum:

- THF parsing, typing, lambda/application normalization, and clausification;
- higher-order matching and complete-set-of-unifier enumeration;
- argument congruence, negative and positive extensionality, extensional
  superposition, equality resolution, and equality factoring;
- primitive enumeration, choice instantiation, Leibniz elimination,
  injectivity recognition, local rewriting, and argument pruning; and
- feature classification, automatic scheduling, proof recording, and
  independent proof-checker coverage.

Source presence is not treated as runtime coverage. Each mechanism must be
connected to the proof-search path and either observed in tests/telemetry or
classified as disabled by the effective strategy.

## Corpus and taxonomy

The immutable CASC-30 manifest supplies all 500 THF problems: 400 `TEQ` and
100 `TNE`. Its complete-family train, validation, and test partition is the
only split authority.

The full-corpus audit first performs syntax-only parsing and a short,
fixed-budget automatic search. A pinned, local-only Vampire reference run is
used for failure attribution, not as a proof oracle or distributable
dependency. Every coordinate is classified as one of:

1. syntax/typing rejection;
2. preprocessing or initialization diagnostic;
3. contradictory or unsupported status;
4. solved by Umlaut;
5. search-limited but solved by the reference;
6. jointly search-limited; or
7. externally timed out or crashed.

Static source tags for lambdas, applied variables, choice, equality, and
Boolean/function-valued terms are reported only as explanatory covariates.
They do not override observed outcomes.

## Scoped gap and calibration

The source audit identified an independently testable positive-extensionality
dispatch defect inherited from E: `--pos-ext` is currently guarded by the
negative-extensionality setting. The correction makes the positive rule obey
its own option and leaves both defaults off.

Focused regression tests must demonstrate that positive extensionality works
with negative extensionality disabled, remains disabled when requested, emits
its own count, and proves a pointwise-equality THF problem through a
checker-visible `pos_ext` derivation.

Calibration uses a deterministic, family-disjoint train sample spanning
`TEQ`/`TNE` and manifest difficulty bands. It compares the unchanged
automatic baseline with:

- positive extensionality on all eligible literals;
- positive extensionality on maximal literals;
- positive and negative extensionality together;
- bounded pragmatic primitive enumeration;
- bounded defined-choice instantiation; and
- bounded multi-unifier enumeration.

At most two mechanisms advance. Reproducible coverage is ranked first, then
median solved CPU, generated clauses, high-water clauses, term storage, and
RSS. A mechanism with a contradictory status, proof failure, or more than
1.25 times baseline median high-water clauses does not advance.

## Held-out evaluation

Validation selects at most one candidate without inspecting test. Held-out
test compares the frozen candidate with the automatic baseline at short and
larger budgets, with two repetitions. A separate FOF control sample from
`FEQ`, `FNE`, and `UEQ` measures the same candidate's fast-path overhead.

The report includes:

- coverage and unique solves by category, family, and difficulty band;
- parsing and failure-taxonomy counts;
- generated, processed, high-water, term-storage, and resident-memory ratios;
- observed positive/negative extensionality and related inference counts;
- proof/model polarity disagreements; and
- independent semantic-checker results for every reproducible larger-budget
  proof claim.

The positive-extensionality dispatch correction may remain as an option
correctness fix if its focused tests and all repository gates pass. No
higher-order mechanism is enabled by default unless held-out THF loses no
reproducible solve, all checked proofs verify, FOF median CPU and high-water
ratios are at most 1.02, maximum RSS is at most 1.05, and either:

1. it contributes at least two test-only reproducible solves with no
   baseline-only solve; or
2. common-solved THF median CPU is at most 0.95 with generated and high-water
   clauses at most 1.02.

Otherwise the default schedule remains unchanged and the measured gap is
reported for future portfolio work.

## Secondary positive-extensionality holdout

This secondary protocol was added after staged validation selected
`choice_depth1`, but before any positive-extensionality result was run on the
test families. The staged outcomes are therefore known; the secondary
positive-extensionality coordinates are not.

The same frozen candidate runs the unchanged automatic baseline and
`--auto --pos-ext=all --neg-ext=off` on all 30 THF test problems, twice, at a
10-second soft and 12-second hard CPU budget with proof objects. The same pair
runs the 18-problem `FEQ`/`FNE`/`UEQ` control, twice, at the existing 5/7
second budget. This audit is diagnostic: it can validate the option fix and
quantify its inference/search growth, but it cannot retroactively promote the
mechanism or change the preregistered default decision.

Nörgler 1.1 with the pinned original-E higher-order backend is the adopted
THF semantic checker. A hashed checker view may replace a file-cited proof leaf
with its exact source only when variable incidence and the non-parenthesis
token stream are unchanged; Nörgler must then positively verify that source
leaf and semantically re-prove every descendant. Checker `Error`, `Unknown`,
or timeout remains a coverage gap, never proof success.

## Implementation

The defect was in `compute_ho_inferences`: the positive-extensionality branch
tested `neg_ext` before calling `compute_pos_ext`. It now tests `pos_ext`.
Defaults remain unchanged. The correction also adds:

- a `PosExts` proof-state statistic and
  `inferences.positive_extensionality` search-telemetry field;
- focused generation tests for on/off behavior and the independent option
  gate;
- an end-to-end THF refutation whose proof contains
  `inference(pos_ext,[status(thm)],...)` while `NegExt=0`; and
- corrected `arg-cong`, `neg-ext`, and `pos-ext` option diagnostics.

The independently checked focused input uses fully parenthesized THF
application equality because Nörgler 1.1 otherwise parses the source with a
different precedence than its canonical proof rendering.

## Capability and runtime audit

The source and production-path audit found no missing top-level branch in the
represented higher-order dispatcher. The high-value gaps are behavioral,
scheduling, parser/preprocessing breadth, and checker coverage rather than an
absent dispatcher:

| Capability | Umlaut production state | Evidence and remaining gap |
| --- | --- | --- |
| THF parsing, types, lambda/application preprocessing | connected | The 500-problem audit still has 120 syntax/typing rejections and 126 preprocessing/initialization diagnostics. |
| Higher-order matching and CSU enumeration | connected | `multi_unif8` was calibrated but excluded at a 1.9012 median solved high-water ratio. |
| ArgCong, NegExt, PosExt | connected | The PosExt option gate was defective and is now independent; the focused proof has PosExt=1 and NegExt=0. |
| ExtSup, ExtEqRes, ExtEqFact | connected through proof-state extension indexes | No missing dispatcher branch was found; no default change was evaluated separately. |
| Primitive enumeration and choice instantiation | connected and opt-in | Both were calibrated; `choice_depth1` won validation but had no held-out coverage or search-size difference. |
| Leibniz elimination and injectivity recognition | connected | No independent high-value scheduling signal appeared in the audit. |
| Proof recording | connected | The focused `pos_ext` step is checker-visible and Nörgler verified it. |
| General THF semantic checking | partial | Nörgler verified the focused axiom-only refutation, but 0/22 held-out theorem proofs: 16 adapter-scope gaps and 6 checker implementation gaps. |

The bundled E source confirms the inherited two-option gate. The correction is
an intentional Umlaut improvement rather than compatibility preservation.
The pinned Vampire executable was used only as a short-budget reference for
the failure taxonomy; its local VIRAS license boundary prevents distribution
and it is not a proof oracle or product dependency.

## Full-corpus audit

All 500 CASC-30 THF problems completed and then resumed 500/500 from the exact
contract. At the fixed 2/4-second audit budget:

| Classification | TEQ | TNE | Total |
| --- | ---: | ---: | ---: |
| Umlaut solved | 130 | 30 | 160 |
| Search-limited, Vampire solved | 31 | 2 | 33 |
| Jointly search-limited | 48 | 13 | 61 |
| Preprocessing/initialization diagnostic | 83 | 43 | 126 |
| Syntax/typing rejection | 108 | 12 | 120 |

The audit contract is
`e3835f039b6268b7d13d3051c8e4be95124a9c46e0bbde8db284fd34b94d14d9`;
its report ID is
`24b139c7b910ae868cb3e4c7c3ed9588e99813de0f12624a9e1830bde1a32955`.

## Staged and secondary results

The frozen candidate binary has SHA-256
`4b1d7c264eabfb5ce4e7867e65e5fdd26e3270697044b335be68809cb13b1972`.
Fresh execution and exact resumption passed for every phase:

| Phase | Problems | Runs | Selection/result |
| --- | ---: | ---: | --- |
| calibration | 45 | 315 | `choice_depth1`, `pos_neg_ext_all` advanced |
| validation | 27 | 162 | `choice_depth1` selected |
| test | 30 | 240 | 11 solves for selected and baseline at the larger budget |
| staged FOF control | 18 | 72 | 6 solves for selected and baseline |
| direct PosExt holdout | 30 | 120 | 11 solves for PosExt and baseline |
| direct PosExt FOF control | 18 | 72 | 6 solves for PosExt and baseline |

There were 981 controlled search runs and zero contradictory terminal
statuses. At the larger held-out budget, `choice_depth1` versus baseline had
no unique solve and paired all-run CPU/generated/high-water/term-storage/RSS
ratios of 1.000054/1.0/1.0/1.0/1.005383. Its FOF ratios were
1.000121/1.0/1.0/1.0/1.000064.

Direct PosExt versus baseline also had no unique solve. Its THF paired ratios
were 1.000014/1.0/1.0/1.0/0.999994, and its FOF ratios were
0.999913/1.0/1.0/1.0/1.003214. Positive extensionality fired in only two
held-out run records, once in each, without changing coverage or measured
search size.

## Independent proof checking

ProofCheck 1.0 self-certified but is first-order-only: two held-out THF checks
exhausted the 120-second external limit, and the focused proof was rejected at
its first THF type/application leaf. That path was stopped rather than running
22 known-incompatible serial checks.

The pinned Nörgler 1.1 JAR has SHA-256
`29e9f5210fe9908c50cdc15f305bf08ae6930c0e768cd9eb42ae1ccd8ae1c6bf`.
Its semantic backend is original E higher-order at commit
`17026b1bfe61aaf223cfaae54947c8d2679c31a0`, executable SHA-256
`50a1ce2444c136f737cdc504233b32e7471de33339d9d2fc963d36ff8a02796a`.
Both were built and retained only on the disposable Linux worker.

Nörgler returned `VerifiedGood` for the focused PosExt=1, NegExt=0 proof,
including semantic re-proving of the `pos_ext` step. General held-out coverage
remains incomplete: its conjecture path reaches an explicit
`NotImplementedError` for checker-compatible leaves, while other proofs exceed
the audited adapter's exact-source token contract. These are checker coverage
gaps, not evidence that the Umlaut proofs are unsound.

## Decision and artifacts

Retain the positive-extensionality option correction: it restores the stated
CLI behavior, passes the focused semantic proof check, creates no contradictory
status, and leaves defaults off. Do not enable any evaluated mechanism by
default. Neither selection path adds held-out coverage or a 5% common-solved
CPU improvement, and the required all-held-out-proof verification gate is not
met.

Final validation passed Rust formatting, strict all-target/all-feature Clippy,
all 4,445 library tests and every integration target, and the optimized
all-feature release build. The final release binary has SHA-256
`b02b7663b60021111b4c2db2803df6a334600a6e976d478e065691316287cb80`;
its PosExt=1/NegExt=0 telemetry/proof smoke also returned Nörgler
`VerifiedGood`.

The committed summaries are:

- `audit-summary.json`, SHA-256
  `e636d30787e8d6b16cab230de8c21a4eb3cfd1b0dac44823cd55671fca89f239`;
- `proof-validation.json`, SHA-256
  `05787335892eb023026a28216555ce989f8df1cbed450f1c96e64cb26c7aacca`;
- `results-summary.json`, SHA-256
  `959180775f4856e2c200e1391402f951b390bfa36c2f41273977dbfaa833bcc2`.

The ignored raw archive is
`.artifacts/experiments/2026-07-28-010-higher-order-gap-audit/raw-results.tar.gz`,
18,447,606 bytes, SHA-256
`949f75aff69d7dd309ba9e968f42e64c5451ac9403241c8413e67b547a5043fd`.
It excludes third-party binaries.
