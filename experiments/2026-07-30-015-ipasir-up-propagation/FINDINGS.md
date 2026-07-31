# Findings: IPASIR-UP-style theory propagation

## Outcome

The simulation is correct and deterministic, but the frozen decision is
**defer**. Eager external propagation did not reduce enough Boolean search to
justify its callback and reason-management cost.

No production code, SAT backend, proof path, feature default, or automatic
schedule changed.

## Correctness

The final Ubuntu run covered 100 generated cases—50 SAT `4 -> 4` and 50 UNSAT
`4 -> 3` pigeonhole instances under treatment-blind atom permutations—plus
three hand cases. Lazy final-model checking, partial conflict checking, eager
propagation, the fully encoded reference, and the exhaustive finite oracle
agreed on all 103 cases in both repetitions.

The first repetition logged 3,651 external reason events and exactly 3,651
root backtracks. Every binary reason:

- contained only observed variables;
- represented a valid at-most-one pair;
- contained the advertised propagated literal when applicable;
- was false for a conflict or unit for a propagation under the recorded
  assignment; and
- replayed against an ordered pre-backtrack trail and an empty post-backtrack
  trail.

All five reason/trail mutations were rejected, nine focused tests passed on
Ubuntu, no treatment hit the one-million-step limit, and both semantic hashes
were identical:

```text
ee0b207adaf08dd1ce675647fc871d5c8975d459752dde0e58edf31f891c6a7f
```

## Search and cost

On the 50 generated UNSAT cases:

| Treatment | Boolean decisions | Ratio vs propagation |
|---|---:|---:|
| lazy final-model conflicts | 5,867 | `1.1368` |
| partial conflict-only | 5,636 | `1.0920` |
| eager propagation | 5,161 | `1.0` |
| fully encoded reference | 265 | `0.0513` |

Propagation improved 40/50 cases versus lazy checking, exactly meeting the
80% incidence condition, but only 35/50 versus partial conflict checking. Its
aggregate ratios were `0.8796659281` versus lazy and `0.9157203691` versus
conflict-only, far above the frozen `0.30` and `0.70` gates.

Median aggregate propagation/conflict elapsed ratio was `1.3513182266`, within
the 1.5 overhead guard but still a material cost for only an 8.4% decision
reduction. The fully encoded reference was both much smaller and faster on
this compact theory. The experiment therefore does not support a live
bidirectional callback layer for this workload.

## Interpretation

The result separates three facts:

1. The minimal assignment/reason/backtrack contract can be implemented and
   independently replayed.
2. Detecting conflicts on partial assignments is only modestly better than
   final-model-only communication here.
3. Eager propagation adds another modest reduction, but nowhere near enough to
   beat the complexity of simply encoding these binary theory constraints.

The prior SATCheck captures cannot rescue this result because they contain
locally numbered propositional clauses without stable atom meanings or theory
provenance. A future revisit needs a real arithmetic or AVATAR trace whose
theory constraints are too large or dynamic to encode economically. It would
also need a live CaDiCaL callback prototype, externally added-clause proof
integration, stable production atom identities, cancellation, and bounded
reason lifetime.

## Evidence

- Report: 3,350 bytes, SHA-256
  `14c3869ee6dca0d7bc7430f4c7acb3ee051273fe23dc84e04a7e4b75d60324bc`
- 412 deterministic treatment traces: 124,407 bytes, SHA-256
  `f7561b989412676bbd447d5dc8a3126d0d4dab972b4fd5f5851f06add2e06ddb`
- Evidence archive: 133,194 bytes, SHA-256
  `170eaafa4c9a4ee3a04196d5fe1fb3d70bfe671c93b3f60e24a4a9336da8b1cc`
- Corpus description SHA-256
  `11c6b9f8489ecdf633a14ea84bcc652f4b64209574abb032478fdd00f2d8d467`

The ignored artifacts are retained under
`.artifacts/experiments/2026-07-30-015-ipasir-up-propagation/`.
