# Findings: conflict-driven VIRAS feasibility

## Outcome

The preregistered finite affine prototype is **supported**, but production
CD-VIRAS is **deferred**.

All correctness, determinism, search-reduction, overhead, and corpus gates
passed. That result is narrower than a production recommendation: the focused
treatment won by invoking a complete exact affine feasibility checker during
clause minimization, and every one of its 203 learned clauses simplified to
the empty clause. In other words, the strongest control recognized global
linear-arithmetic UNSAT; it did not demonstrate that general VIRAS
false-interval or infinity learning is ready.

No production code, feature default, or automatic schedule changed.

## Frozen corpus and correctness

The final Ubuntu run used the frozen seed `0x43445649524153`, 300 generated
cases, eight hand-authored boundaries, and two complete repetitions. The
generated set contained 100 SAT and 200 UNSAT formulas. Six hand cases were in
the supported finite equality-guarded affine slice; the epsilon and periodic
boundaries failed closed as preregistered.

All three treatments agreed with the independent exact rational affine
checker and pinned Z3 on every supported case. The two semantic result hashes
were identical:

```text
d47d970d27bdeaabdfbe289f95cb0f8c41e72e4af25717ec3e3822ff812db68d
```

The searches inserted 8,961 learned-clause occurrences. Per-case
deduplication produced 8,758 unique implication obligations, and pinned Z3
returned `unsat` for all of them. Together with the 306 supported original
formulas, the incremental Z3 session checked 9,064 queries with zero
disagreements. The experiment-local exact checker independently rechecked
every inserted clause, every progress check passed, and all four mutation
probes were rejected. Fourteen focused tests passed on Ubuntu.

## Search and cost

Across all 308 cases, the first repetition recorded:

| Treatment | Substitutions | Leaves | Inserted clauses | Learned prunes |
|---|---:|---:|---:|---:|
| eager | 76,677 | 31,451 | 0 | 0 |
| basic | 9,085 | 8,861 | 8,758 | 6,806 |
| focused | 1,423 | 306 | 203 | 893 |

On the 200 generated UNSAT cases, focused search used 888 substitutions versus
8,545 for basic search, a ratio of `0.1039204213`, and improved every case.
Basic search used fewer substitutions than eager search on 283 of 300
generated cases.

Search-count improvement did not make basic CD-VIRAS cheaper. Its measured
times were 178.746 and 178.354 seconds, versus 113.573 and 131.802 seconds for
eager enumeration. Full-path clause generation and exact validation cost more
than the substitutions they avoided. Basic clauses had median length four and
maximum length seven.

Focused search took 4.284 and 5.024 seconds, only `0.0261` times basic's median
elapsed time. However, all 203 focused clauses were empty: deletion
minimization proved each entire UNSAT conjunction infeasible. This is useful
architecture evidence—an exact affine conflict oracle can dominate the
candidate tree—but it is not evidence that richer CD-VIRAS learning pays for
itself.

## Production decision

The experiment closes the Bead as a successful bounded feasibility study and
keeps production deferred for three reasons:

1. Basic CD-VIRAS reduced substitutions but was slower than eager base VIRAS.
2. The winning focused control is effectively a complete QF-LRA decision
   procedure embedded in conflict analysis; a production design should assess
   that theory solver directly rather than credit the gain to VIRAS learning.
3. The prototype deliberately does not implement or validate epsilon
   false-interval lemmas, aperiodic infinity lemmas, periodic residue lemmas,
   epsilon-plus-infinity lemmas, learned `Z`-grid search, or the general
   multi-variable context-lifting invariant.

Those omitted branches are the distinctive and highest-risk parts of
CD-VIRAS. Production work should not begin until a future demand trace shows
that they matter and a proof-object design can validate them independently.

## Reproducibility

The final report is 14,520,957 bytes with SHA-256:

```text
d104fa3a5ad209e4874a3803118157feb53624d444bba81390bed246c0562d46
```

The 924 complete first-repetition treatment traces are stored in a
9,600,826-byte deterministic gzip stream with SHA-256:

```text
b132354def872e493ade805af1e1ed9a09da4ecc42554f82c4b464a1e436683e
```

The complete 8,318,994-byte evidence archive has SHA-256:

```text
6429c5b387370d877575a309295f632e8c89b11ca41594fbe5a322d2453a6fdf
```

It is retained under
`.artifacts/experiments/2026-07-30-014-conflict-driven-viras/`. The final run
used Z3 `5.0.0`, executable SHA-256
`f331d9f5953deaf88a900f83b45a62a7e3d63319a8dd89ca59c53abe02616bf9`.
The corpus description hash is
`1cbe95a1281b2cb8ac758c791b11791a3b7367f3c7946fe430b10505608bc372`.

An initial artifact-free full attempt was stopped after the harness retained
several gigabytes of trace objects. The only correction streamed
first-repetition traces directly into deterministic gzip. A two-repetition
smoke run before and after the correction had identical semantic and trace
hashes; the frozen treatments and gates did not change.
