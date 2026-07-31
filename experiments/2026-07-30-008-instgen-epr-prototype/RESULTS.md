# Inst-Gen-style comparison results

The frozen matrix contains 47 coordinates over 29 problems: one run for each
of 11 train problems and two runs for each of 18 validation/test problems.

| Method | Reproducible held-out solves | Unique solves |
| --- | ---: | ---: |
| saturation | 7 | 0 |
| standalone instantiation | 4 | 0 |
| equal-budget portfolio | 7 | 0 |
| cooperative instantiation + saturation | 7 | 0 |

The three seven-solve methods solved the identical set:
`PUZ001-3`, `PUZ018-2`, `PUZ028-1`, `PUZ028-2`, `PUZ028-4`,
`PUZ036-1.005`, and `PUZ037-2`. Standalone instantiation solved the four
`PUZ001/028` satisfiable cases. Every held-out method/problem outcome was
stable across both repetitions.

| Method | Verified held-out coordinates | Median user CPU (s) | Maximum RSS (KiB) | Refutation bytes |
| --- | ---: | ---: | ---: | ---: |
| saturation | 14 | `0.015000` | 167,300 | 1,164,858 |
| standalone instantiation | 8 | `0.065981` | 21,880 | 0 |
| equal-budget portfolio | 14 | `0.192985` | 167,296 | 1,164,858 |
| cooperative | 14 | `0.182985` | 167,288 | 1,087,284 |

Across all long and short instantiation runs, the candidate made 465 SAT
calls, performed 371 refinement iterations, generated 10,374 unique ground
instances, and enumerated 14,570,530 substitutions. It returned 22 complete
checked models and 72 UNKNOWN results. It returned no UNSAT result, so the
measured candidate DRAT proof count and proof bytes are both zero.

Cooperation exchanged 4,353 replayed instances in 36 coordinates. It added and
lost zero solve versus both saturation and portfolio. On common verified
coordinates, cooperative/portfolio median user CPU was `0.995075`, maximum
proof-byte ratio was `1.000353`, and maximum RSS ratio was `1.005783`.
Against the independent saturation worker, the cooperative median user-CPU
ratio was `18.298500` over the ten coordinates with nonzero measured baseline
CPU.

The preregistered result is `leave_production_unchanged`.
