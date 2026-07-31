# Findings: E-matching and model-counterexample instantiation

Bead: `E_Rust_Port-9jt.6.6`

Decision: **stop; leave production unchanged**

## Executive result

The narrow function-free EPR prototype is sound on every checked result but
does not justify an E-matching worker. Across 47 real corpus coordinates,
bounded clausification, trigger matching, and model-counterexample
instantiation produced respectively 12, 9, and 11 verified terminal answers.
On reproducible held-out problems, clausification and MBQI each solved four;
E-matching solved three, added none, and lost `PUZ028-2`.

E-matching also failed the frozen efficiency alternative. On the three common
held-out solves it retained 678 clauses versus MBQI's 330, an instance ratio
of 2.055. Its median common-coordinate wall ratio was 0.693, but the gate
required retaining every MBQI solve and using at most half as many instances.
Bounded clausification contained every E-matching solve.

The frozen reproducibility gate independently forces `stop`: nine held-out
method/problem pairs had different semantic instance traces between their two
runs. All terminal statuses remained stable and every individual certificate
validated, but wall-limited prefixes and termination reasons were not
deterministic.

## What was evaluated

The source design followed the separation visible in the pinned Z3 checkout:

- `smt_quantifier.cpp` combines unary and multipattern E-matching with
  restart-aware MBQI; and
- `smt_model_finder.cpp` constructs instantiation sets and repairs candidate
  models.

The prototype intentionally tested only equality-free, function-free,
untyped CNF. Its universe was the finite set of source constants. Every method
started from the same first-constant instance of each source clause and used
the same CaDiCaL 3.0.1 adapter. The treatments were:

1. stable bounded complete clausification;
2. deterministic first-covering-atom or greedy-multipattern matching over
   ground atoms from prior rounds; and
3. batches of at most 64 instances falsified by each candidate model.

Each real treatment had four search seconds, 100,000 unique-clause slots, and
250,000 substitution or match steps. The corpus was the exact treatment-blind
29-problem EPR corpus from Experiment 008: train ran once and
validation/test twice, for 47 real coordinates. Five hand cases exercised
ground contradiction, unary rounds, satisfiable propagation, a multipattern
join, and an incomplete trigger.

This is a controlled approximation of Z3's architecture, not a claim to
reimplement Z3. Equality-aware E-graphs, positive-arity functions, types,
pattern inference scoring, relevance, lazy rematching, model projection, and
quantifier-specific restart policies are outside the boundary.

## Correctness evidence

The matrix contains 156 independently checked certificates: 47 real plus five
hand coordinates, each under three treatments. The verifier replayed
3,528,665 retained ground clauses as substitutions of their original source
clauses. It additionally checked:

- 724,650 trigger-generated instances across 217 rounds;
- 5,219 inferred trigger records;
- 258 candidate-model refinement records containing 817 added
  counterexamples;
- 37 exhaustive finite Herbrand models; and
- nine DRAT proofs from the hand UNSAT cases.

There were no terminal polarity disagreements and no certificate-validation
failures. The hand integration suite checked all 15 hand
treatment/instance combinations. Mutations to a substitution, ground clause,
trigger binding, refinement model, DIMACS clause, and DRAT proof all failed
closed. Five focused unit tests cover trigger inference, multipattern joins,
repeated-variable consistency, and semantic hashing.

No real UNSAT problem terminated within the frozen bounds, so the real-corpus
comparison is about verified SAT answers and sound UNKNOWN prefixes. The hand
corpus supplies the exercised UNSAT proof path.

## Search and instance results

| Method | Verified real coordinates | SAT | UNSAT | UNKNOWN | Retained instances |
| --- | ---: | ---: | ---: | ---: | ---: |
| clausify | 12 | 12 | 0 | 35 | 2,794,205 |
| ematch | 9 | 9 | 0 | 38 | 729,112 |
| mbqi | 11 | 11 | 0 | 36 | 5,278 |

The reproducible held-out solve sets were:

- clausify: `PUZ001-3`, `PUZ028-1`, `PUZ028-2`, `PUZ028-4`;
- ematch: `PUZ001-3`, `PUZ028-1`, `PUZ028-4`; and
- mbqi: `PUZ001-3`, `PUZ028-1`, `PUZ028-2`, `PUZ028-4`.

This MBQI set exactly repeats Experiment 008's four standalone held-out solves.
It therefore adds no new complementarity evidence to that experiment's
already-negative saturation/cooperation result.

The E-matcher attempted 1,108,383 complete matches over 207 real-corpus
rounds. It encountered 352,693 duplicate ground clauses, reached a syntactic
fixed point in 31 of 47 coordinates, and exposed eight first model
counterexamples that its trigger fixed point had not generated. The corpus
used 3,247 inferred unary patterns and 160 multipatterns, with maximum
multipattern size four. MBQI made 250 SAT calls, performed 204 refinement
iterations, and enumerated 7,592,639 substitutions.

## Reproducibility failure

All paired held-out statuses agreed, but nine semantic hashes differed:

- E-matching on `PLA031-1.007`, `PLA031-1.008`, `PUZ036-1.005`,
  `PUZ037-2`, `PUZ052-1`, and `SWV420-1.020`;
- clausification on `PUZ018-2` and `SWV419-1.035`; and
- MBQI on `PUZ018-2`.

Most differences are different wall-limited prefix sizes. Two pairs retained
byte-identical instance sets but reported different stopping boundaries:
`SWV419-1.035` clausification ended as solver-timeout versus instance-limit,
and `SWV420-1.020` E-matching ended as solver-timeout versus
matching-timeout. This is not a logical disagreement, but it falsifies the
preregistered trace-determinism requirement and shows that a future worker
would need deterministic work checkpoints rather than wall-clock arbitration.

## Interpretation

Within this fragment, MBQI is much more selective. Trigger matching eagerly
materializes atoms that are syntactically reachable even when they do not
repair the current candidate model, while the model loop generates only
observed counterexamples. E-matching's lower common-solve wall time does not
compensate for losing a solve and retaining twice as many clauses on common
held-out problems.

The result does not rule out E-matching for typed SMT formulas, equality-heavy
terms, or finite theory models. Those are exactly the mechanisms excluded here
and would need a separate corpus and proof architecture. It does establish
that adding a generic trigger worker to Umlaut's current EPR/saturation path is
not supported by this evidence.

Production code, default schedules, and command-line behavior remain
unchanged.

## Reproduction and artifacts

Run the focused tests with:

```text
python3 -m unittest discover \
  -s experiments/2026-07-30-016-ematching-mbqi-feasibility \
  -p 'test_*.py' -v
```

The frozen protocol is in `PREREGISTRATION.md`. `run_experiment.py` performs
the sequential matrix, while `run_shard.py`, `resume_parallel.py`, and
`merge_results.py` provide fail-closed resumption without concurrent JSONL
writes. `analyze.py` applies the frozen decision gates and
`audit_results.py` produces validation and repeat diagnostics.

The retained Ubuntu artifact directory is
`.artifacts/experiments/2026-07-30-016-ematching-mbqi-feasibility/`.
Important source-artifact hashes before packaging were:

- results JSONL:
  `b1a7ccadb9262ab9400a4b50326d85ea85215414e451f345e3b35a4fd73deeab`;
- analysis JSON:
  `63010b6e1504947f72d64b2d4d6638f823bb3db02f7716bb07d3c4750de702c8`;
- validation JSON:
  `6629fddf2196e461280c8b3c5a6facc51a2883eff8ca36c94af481de895a26d9`;
  and
- semantic trace:
  `281d34fe94042b3397ac61cd52c83a8eaa9d91af59a3c5cef5891a41a1eff5f0`.
