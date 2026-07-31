# TSM ranking feasibility findings

## Verdict

`reject` for automatic-schedule adoption and for further label collection.

The measured index-comparison optimization is retained because it removes a
large, semantics-preserving cost from the existing opt-in TSM path:

- native validation scoring fell from 103.027 to 10.773 CPU microseconds per
  weighted occurrence, an 89.5% reduction and a pass of the frozen
  50-microsecond gate;
- identical-work live learned/control CPU fell from 6.211 to 1.620, but remains
  above the frozen 1.10 gate; and
- learned-only live-search instructions fell from 957,161,889 to 102,391,457,
  an 89.3% reduction.

The failed live-cost gate selects `reject` before the conditional common-solve
and control-only label phases. No new held-out problems were run, no labels
were collected or fabricated, and automatic schedules remain unchanged.

## Inputs and isolation

The phase-isolated workload reused Experiment 018's immutable artifacts:

- complete prior archive SHA-256
  `8af1871793377c79de79dce89cdcbd5ec8487725490e0c7a8891682999890156`;
- proof-derived knowledge-base tree SHA-256
  `838a4f14137344c8d1c0c17a0503fb8fc0a136dbcb206b35f6927c898fe7d13f`;
- full validation classifier input SHA-256
  `462f547dea47b6bb5fa42ee6966aa9794878c58399762181a3a00b9be0239fe4`;
- generated empty-test input SHA-256
  `88e9f3ab96e6462de3253b2b61571d14f7055305818cc9f89f838e997f9d40ca`;
  and
- `LCL026-10.p` SHA-256
  `b0e8c769ae659ad7d89f632be19849c9bcdb0c9a34e72380466a8c7eaa556111`.

The controller preserved the validation input's exact training prefix and
replaced only its `Test:` set for the empty phase. It used one warm-up and 11
alternating paired classifier repetitions, seven alternating paired search
repetitions, and fresh Callgrind processes. The live diagnostic stopped both
treatments at 128 processed clauses.

## Baseline attribution

The native baseline executables were:

- `umlaut` SHA-256
  `cbfec550d0b135d9f3ae0233c6b385f6ffec8305ee5fbc36e8b3ed19ca4f5be1`;
  and
- `umlaut-tsm-classify` SHA-256
  `f6167f58ec377f7ff16f948e967f339e5d9e17977c5a5689d05ac2de6793b014`.

The baseline was the `d3f54901` main source state before the two TSM source
files were edited. The runner's original packed snapshot was rooted at
`77a42527`; the intervening uploaded adaptive-checkpoint source did not touch
the TSM comparison or index path. Executable hashes are the authoritative
measurement identities.

Callgrind measured:

| Phase | Baseline instructions |
| --- | ---: |
| Empty classifier | 443,913,366 |
| Full classifier | 591,437,766 |
| Scoring delta | 147,524,400 |
| Scoring delta / weighted occurrence | 983,496 |
| Control search | 120,035,976 |
| Learned search | 1,077,197,865 |
| Learned-only search delta | 957,161,889 |

In learned search, `compute_in_bank` accounted for 659,149,588 inclusive
instructions, `find_tsa_for_term` for 574,186,188, and
`PatternSubst::clone` for 521,292,437. The last figure was 54.5% of the
learned-only delta, clearing the preregistered 25% optimization-eligibility
threshold. Allocation, copying, and destruction were correspondingly
prominent.

## Optimization

`TSMIndex` previously stored normalized terms in a `BTreeSet<IndexTerm>`.
Every lookup constructed an owned query and every comparison cloned both
`PatternSubst` values because the pattern comparison API accepted mutable
substitutions only to lazily populate signature alpha ranks.

The retained safe-Rust change:

1. makes pattern comparison borrow immutable substitutions and obtains the
   exact same rank with the non-mutating `Signature::alpha_rank`;
2. stores the term index as a sorted `Vec<IndexTerm>`;
3. performs borrowed binary search against `(Term, PatternSubst)` parts,
   creating an owned entry only on insertion; and
4. preserves dense keys as insertion-order values even when a new sorted
   position precedes existing entries.

This moves the uncommon model-build insertion to `Vec::insert`, while the hot
lookup remains logarithmic and allocation-free with respect to the query.
The regression test inserts terms in deliberately non-sorted order and
confirms both strict sort order and stable dense keys.

## Candidate results

The exact final native executables were:

- `umlaut` SHA-256
  `8c093b91e7e0de5f37d2f8066199f9b57aaea3a1041f9fa9eb21d116ae1decda`;
  and
- `umlaut-tsm-classify` SHA-256
  `45744fc14c7e590fc52a6fb391ba517baaa30f84fcbe8c0d06d9dda25580665d`.

The exact final symbol-rich Callgrind executables were
`30ccdfadfe0b71f8967590b24a9595b421f7f363f73da9f1fdd82e83dd7468f3`
and
`301b56bbe6fbbf27defe7cd732e320f407c3d54104fe989587621364c2f97ae9`,
respectively.

| Metric | Baseline | Candidate | Gate |
| --- | ---: | ---: | --- |
| Native scoring CPU / weighted occurrence | 103.027 us | 10.773 us | `<=50`: pass |
| Native learned/control CPU | 6.211 | 1.620 | `<=1.10`: **fail** |
| Callgrind scoring instructions / occurrence | 983,496 | 108,514 | diagnostic |
| Callgrind learned/control instructions | 8.974 | 1.853 | diagnostic |
| Callgrind learned-only instructions | 957,161,889 | 102,391,457 | diagnostic |

In the candidate learned profile, `compute_in_bank` fell to 86,475,614
inclusive instructions and `find_tsa_for_term` fell to 1,103,324. The
remaining 46,970,997 inclusive `PatternSubst::clone` instructions belong to
pattern normalization outside the removed index-comparison clone.

All classifier outputs were byte-stable. Candidate full-classifier stdout
SHA-256 was
`36be34f46106d2e2fd7bb22a3ff8045cbd30778d94c6d7dd90b3da51dcdab4e1`;
empty stdout SHA-256 was
`817862a686b105990a29489a5bba44e6357d1fad7fe905adec2c50d1b8551b9c`.
Learned and control search output both had SHA-256
`ed23d184b6a47235a98af994861855f4bc1682158a7b27710b9f8966e0246b9a`.

Both search treatments produced the exact same work signature: stopped at 128
processed clauses, 2,682 paramodulations, 449 generated non-trivial clauses,
104 processed non-trivial clauses, 24 trivial clauses, and identical
rewriting, subsumption, queue, and high-water counters. This rules out changed
search work as the explanation for the cost reduction.

## Frozen decision and limitations

The candidate missed the native live-cost gate by 47.2% relative to the
allowed ratio (`1.620 / 1.10`). Under the preregistration this prohibits:

- the three-repetition four-problem common-solve phase;
- the candidate-blind control coverage pool;
- new held-out two-class label extraction;
- calibration or held-out complementarity reruns; and
- any automatic-schedule change.

The study therefore does not claim that the Experiment 018 label-scarcity
problem was solved. It establishes instead that query-substitution ownership
was the dominant removable TSM cost, that removing it is correct and useful
for opt-in users, and that additional work would still be required before TSM
ranking is cheap enough to justify spending proof-search budget on labels.

Two setup runs are retained but excluded from the measurements:

- `baseline-native-v1` stopped because the controller initially expected the
  telemetry reason `processed_limit`; production emits the equivalent
  `step_limit`. The controller now accepts both.
- `baseline-callgrind-v1` was interrupted by an external SSH timeout and has a
  zero-length control profile. `baseline-callgrind-v2` completed but used the
  stripped native executable and is retained only as a diagnostic. The
  symbol-rich `baseline-callgrind-debug-v1` is authoritative.

An earlier candidate profile before a Clippy-only `Vec::clone_from` cleanup is
also retained for audit; all reported candidate values above come from the
exact final source.

## Validation

On the Ubuntu 24.04 runner:

- `cargo fmt --all -- --check`;
- default `cargo check --all-targets`;
- default standard and pedantic Clippy with warnings denied;
- default `cargo test --all-targets` (4,497 library tests plus every binary
  and integration target);
- all-feature check, pedantic Clippy, and tests with pinned CaDiCaL 3.0.1
  (4,547 library tests plus every target);
- Windows GNU `cargo check --target x86_64-pc-windows-gnu --all-targets`
  (two pre-existing target-specific dead-code warnings);
- focused pattern and index tests; and
- three Python controller/parser tests and Python bytecode compilation.

The first all-feature check intentionally failed closed when
`UMLAUT_CADICAL_SOURCE` was absent; rerunning with the retained pinned
`/opt/e-rust-port/cadical-3.0.1` source passed the complete matrix.

## Artifacts

The ignored raw archive is:

`.artifacts/experiments/2026-07-30-011-tsm-ranking-feasibility/tsm-ranking-011-evidence-v1.tar.gz`

- size: 14,986,691 bytes
- SHA-256:
  `8891f695519de84907dcf01d05342a30dc32d66663d77729c5769eef7594802e`

It contains native summaries, raw Callgrind profiles and annotations,
machine-readable analyses, exact measured executables, and the disclosed
invalid/setup runs. Experiment 018's separately retained archive is referenced
by hash rather than duplicated.
