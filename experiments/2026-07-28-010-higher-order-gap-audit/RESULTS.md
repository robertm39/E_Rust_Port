# Higher-order gap audit results

- Selected staged strategy: `choice_depth1`.
- Umlaut binary SHA-256: `4b1d7c264eabfb5ce4e7867e65e5fdd26e3270697044b335be68809cb13b1972`.
- Controlled search runs: 981.
- Full-corpus audit: 500 THF problems.

## Full-corpus failure taxonomy

| Classification | Problems |
| --- | ---: |
| `jointly_search_limited` | 61 |
| `preprocessing_or_initialization_diagnostic` | 126 |
| `search_limited_reference_solved` | 33 |
| `syntax_or_typing_rejection` | 120 |
| `umlaut_solved` | 160 |

## Held-out staged winner versus baseline (larger)

- Coverage: choice_depth1 11, baseline_auto 11.
- Left-only: none.
- Right-only: none.
- Paired all-run CPU ratio: 1.000054.
- Paired all-run generated ratio: 1.0.
- Paired all-run high-water ratio: 1.0.
- Paired all-run term-storage ratio: 1.0.
- Paired all-run max-RSS ratio: 1.005383.

## Staged winner FOF control

- Coverage: choice_depth1 6, baseline_auto 6.
- Left-only: none.
- Right-only: none.
- Paired all-run CPU ratio: 1.000121.
- Paired all-run generated ratio: 1.0.
- Paired all-run high-water ratio: 1.0.
- Paired all-run term-storage ratio: 1.0.
- Paired all-run max-RSS ratio: 1.000064.

## Direct positive-extensionality THF holdout

- Coverage: pos_ext_all 11, baseline_auto 11.
- Left-only: none.
- Right-only: none.
- Paired all-run CPU ratio: 1.000014.
- Paired all-run generated ratio: 1.0.
- Paired all-run high-water ratio: 1.0.
- Paired all-run term-storage ratio: 1.0.
- Paired all-run max-RSS ratio: 0.999994.

## Direct positive-extensionality FOF control

- Coverage: pos_ext_all 6, baseline_auto 6.
- Left-only: none.
- Right-only: none.
- Paired all-run CPU ratio: 0.999913.
- Paired all-run generated ratio: 1.0.
- Paired all-run high-water ratio: 1.0.
- Paired all-run term-storage ratio: 1.0.
- Paired all-run max-RSS ratio: 1.003214.

## Inference and proof audit

- Positive extensionality fired in 2 held-out run records (2 total inferences).
- Nörgler verified 0/22 reproducible larger-budget held-out proof claims.
- The focused PosExt=1, NegExt=0 refutation was verified.
- Checker taxonomy: `{'adapter_coverage_gap': 16, 'checker_implementation_gap': 6, 'verified': 1}`.
- Contradictory terminal statuses: 0.

## Decisions

- Option correction: `retain_positive_extensionality_option_fix`.
- Default schedule: `retain_existing_higher_order_defaults`.
