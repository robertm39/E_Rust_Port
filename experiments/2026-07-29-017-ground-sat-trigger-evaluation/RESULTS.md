# Ground-SAT trigger results

| Strategy | Reached coords/problems | Calls | SAT / UNSAT / limited | SAT CPU/call | SAT CPU share | Common solves | CPU ratio | Generated | High-water | RSS | Decision |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| off | 0/0 | 0 | 0 / 0 / 0 | n/a | n/a | 5 | n/a | n/a | n/a | n/a | baseline |
| step5000 | 8/4 | 10 | 10 / 0 / 0 | 0.036845 | 0.4606% | 5 | 1.001514 | 1.000000 | 1.000000 | 0.999448 | keep_default_off |
| step10000 | 2/1 | 2 | 2 / 0 / 0 | 0.118120 | 1.1813% | 5 | 1.003103 | 1.000000 | 1.000000 | 0.998741 | keep_default_off |
| size10000 | 38/19 | 656 | 650 / 0 / 6 | 0.269156 | 55.8187% | 5 | 1.542813 | 1.000000 | 1.000000 | 1.132813 | reject |

Ratios are candidate/baseline medians on paired coordinates for
common reproducible solves. Full detail is in
`results-summary.json`.
