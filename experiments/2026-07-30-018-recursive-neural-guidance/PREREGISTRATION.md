# Recursive neural clause-guidance preregistration

## Bead and question

This experiment addresses `E_Rust_Port-9jt.3.4`.

Can a compact structure-aware recursive encoder rank proof-relevant
given-clause contents better than chronological selection and a simple linear
structural model on family-held-out Umlaut proof traces, with enough
reproducibility and deployment efficiency to justify an online experiment?

This is an offline feasibility study. It neither changes Umlaut production
search nor treats an optimistic reranking simulation as an end-to-end prover
result.

## Frozen source and evidence

- Source revision before prototype implementation:
  `a9a5acabdf2e7d7db6ef6b520c63e5debf39097f`.
- Source evidence archive:
  `tsm-learning-018-81232361-complete.tar.gz`.
- Archive SHA-256:
  `8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156`.
- The nine selected trace members, their hashes, families, splits, row counts,
  and positive counts are frozen in [`trace_manifest.jsonl`](trace_manifest.jsonl).
- The traces predate this neural candidate. Earlier TSM work used all four
  held-out traces as its validation evidence, so the neural validation and test
  families are candidate-blind but not globally unseen.

The family-disjoint split is:

- train: `MGT`, `SWW`, `LAT`, `KLE`, and `SYN` (2,918 clauses, 154 positives);
- validation: `LCL` (1,298 clauses, 72 positives); and
- test: `PUZ` and `ROB` (343 clauses, 51 positives).

Only train and validation results may be inspected before the frozen
validation decision is written. Test is then evaluated unchanged, without
parameter selection.

## Frozen extraction and labels

Each `%cnf(...)` comment before the final proof is one chronological
given-clause observation. A proof line whose inference root is `evalgc(...)`
identifies proof-relevant given-clause content.

The extractor parses balanced terms and literals, normalizes whitespace,
variable names by first occurrence, literal order, and symmetric equality,
and labels an observation positive exactly when its normalized clause content
occurs in an `evalgc` proof node. This content-level label intentionally marks
duplicate observations alike; it does not claim to reconstruct a unique clause
identity. Proof `evalgc` contents absent from the chronological log are
reported as unmatched and are never fabricated.

Extraction is invalid if an archive/member hash differs, a trace lacks a
completed CNF refutation, its given/positive/proof-evalgc/unmatched counts
differ from the manifest, or a source observation cannot be parsed.

## Frozen models

All numeric operations use the Python standard library and deterministic
64-bit hashes. No code or model is imported from another prover.

1. `chronological`: original given-clause order; no fitted parameters.
2. `linear`: class-weighted logistic regression over training-normalized
   scalar clause features: literal counts by polarity, syntax-node count,
   maximum term depth, variable occurrences, distinct variables, symbol
   occurrences, distinct symbols, and equality count.
3. `recursive`: the same normalized scalar features plus a 12-dimensional
   frozen recursive representation. Each syntax node receives a seeded
   symbol/arity/polarity vector; child representations are combined in order
   through a deterministic `tanh` recurrence, and literal representations are
   pooled by mean and maximum. A trained 8-unit `tanh` MLP produces the
   ranking score.

The recursive encoder is deliberately a small prototype: its structural
recurrence is frozen and only its ranking head is learned. Advancing it would
authorize a separately preregistered end-to-end trainable encoder study, not
production adoption.

The linear model uses class-weighted binary cross-entropy, full-batch gradient
descent, training-only feature normalization, learning rate `0.03`, L2
coefficient `0.0001`, gradient-norm clipping at `5`, and 300 epochs. It is
initialized to zero and deterministic.

The recursive head uses balanced binary cross-entropy for 160 deterministic
epochs. Every epoch contains all training positives and an equal-size slice of
a seeded shuffled negative cycle. It uses the same training-only
normalization, learning rate, L2 coefficient, and gradient clipping. The
recursive seeds are `[11, 23, 37, 53, 71]`; each seed controls the frozen
embeddings, negative order, and head initialization. No hyperparameter search
is permitted.

## Metrics and simulation boundary

Metrics are computed per problem and macro-averaged:

- average precision (AP);
- ROC AUC;
- positive-versus-negative pairwise ranking accuracy;
- proof-positive recall in the top 1%, 5%, 10%, and 20% of observations; and
- the ranked fraction needed to include every positive.

Ties use chronological order. The last metric is an optimistic offline
proof-cost simulation because it assumes all future generated clauses are
available for reranking; it is not a solve or a faithful model of changed
search trajectories.

For each learned model the study also reports training wall time, serialized
model bytes, peak process RSS, in-process batch inference microseconds per
clause, and checksum-stable repeat inference. The frozen recursive model is
additionally measured through a persistent newline-JSON external process with
batches of 64. Reported external time includes serialization and IPC.

An ONNX-style runtime is recorded as `not-evaluated`: the retained Ubuntu
runner has neither ONNX Runtime nor a numerical package, and adding a runtime
or converter before the offline quality gate would confound the dependency and
package-cost question. No repository dependency may be added by this study.

## Validation gate

The unchanged candidate may reach the frozen test set only if all extraction
and deterministic-repeat checks pass and the median recursive seed meets all
of:

1. validation macro AP is at least linear AP plus `0.03`;
2. validation macro top-10% recall is at least linear plus `0.05`;
3. validation macro all-positive prefix fraction is at most `0.80` times
   linear;
4. at least four of five seeds beat linear AP and top-10% recall;
5. recursive AP seed range is at most `0.10`;
6. in-process batch inference is at most `100` microseconds per clause;
7. external-process batch inference is at most `500` microseconds per clause;
8. serialized model size is at most 1 MiB; and
9. peak RSS attributable to the study process is at most 256 MiB.

If the validation gate fails, test evaluation and all end-to-end prover runs
are skipped. The final result is `stop-offline-validation`; test ranking and
solve counts are reported as `not-run`, not as zero.

## Test and online-advancement gate

If validation passes, all five frozen seeds and the unchanged linear model are
evaluated once on `PUZ` and `ROB`. Online work is authorized only if all
integrity/resource gates remain clean and the median recursive seed meets the
same three quality-effect thresholds against linear on test, at least four of
five seeds beat linear AP and top-10% recall, and test AP range is at most
`0.10`.

Passing yields `advance-online-experiment`. Failing yields
`stop-offline-test`. Neither outcome adopts a model.

Because this study is explicitly gated before integration, end-to-end solve
counts, CPU, and memory are `not-run` unless the full offline gate passes. A
future online experiment must separately preregister candidate generation,
batching, cancellation, equal CPU budgets, proof checking, family-held-out
problems, seed policy, and production/package rollback.
