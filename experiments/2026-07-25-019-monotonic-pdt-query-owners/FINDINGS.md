# Experiment 320: Monotonic PDTree query owners

## Status

Accepted for Bead `E_Rust_Port-j76.5.5`; the normal `1.10x` whole-prover
performance target remains open.

## Question

Can the borrowed first-order PDTree cursor retain each discovered descendant
once per active search instead of rebuilding and dropping the complete owner
set at every safe return?

## Baseline

- Accepted parent: commit `8ed75465`.
- Matched LUSK6 work is `7,662,328,035` Rust instructions versus
  `5,254,418,333` for C (`1.458264x`).
- The borrowed first-order search specialization retires `1,514,251,396`
  self instructions and `1,604,529,380` inclusive instructions.
- Its return-boundary guard retires `115,967,853` instructions across
  `880,523` calls. The accepted implementation reconstructs a replacement
  owner vector and releases the previous owners even when the continuation is
  unchanged.

## Candidate

Keep one monotonic set of descendant owners for the whole active first-order
search:

- the active `PdtSearchState` remains the exact root owner;
- every discovered non-root cursor is acquired once at the safe-call boundary
  and retained in `parked_query_terms` until `record_search_exit`;
- pointer-identity deduplication prevents repeated returns from acquiring the
  same descendant again;
- raw query stacks are cleared before the monotonic owner set and root are
  released; and
- the existing owned higher-order cursor remains unchanged.

This removes the double-buffered scratch owner vector, complete owner-set
reconstruction, and drop/reacquire cycle at each of the `880,523` safe
returns in the matched proof. A focused mutation-between-calls regression
records the detached descendant cursor, changes the root argument, releases
the caller's old owner, and verifies that the second return keeps the same
single parked owner rather than duplicating its reference count.

The production change is confined to `src/clauses/pdtrees.rs`. The focused
candidate source has SHA-256
`ca5b61b6a8b838f95d989e9edfcba10af53c77a195fa99adf46ee2568e7577e2`.

## Method

The focused worker was
`e-rust-codex-260726-094749-cb24` with snapshot
`9a623a404143c004f6725d6b0887158fa0d14774ae282eb17bda7e751cfe1dd3`.
The accepted-parent source archive had SHA-256
`2C7C0ECACD8B0C536CCB2028CCF716D54C957D1F433EB78A47837A84084CF38F`.
The controller lifecycle was:

```powershell
git archive --format=tar --output=accepted-source.tar 8ed75465 `
  src/clauses/pdtrees.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-019-monotonic-pdt-query-owners/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-320
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-019-monotonic-pdt-query-owners/remote_repeat.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-320
}
finally {
    .\linode-runner.ps1 down
}
```

The scripts retain the exact Rustfmt, 44 focused PDTree tests, strict
all-target/all-feature pedantic Clippy, parent/candidate release builds,
Callgrind commands, proof comparisons, and two independent 64-pair native
commands. Exact-source comprehensive validation used fresh worker
`e-rust-codex-260726-100748-dc6e` and snapshot
`8c4d2d3030b8906174718844d0df451fa35dfc6483cb57c7a2cbf6d7280b5046`:

```powershell
.\linode-runner.ps1 run
```

## Falsification criteria

- Every discovered descendant cursor must remain owned through normal return,
  exhaustion, unwinding, caller mutation, and search exit.
- The exact root must remain owned only by the active search state; repeated
  returns must not duplicate parked descendant owners.
- Traversal order, substitutions, backtracking, type/weight constraints,
  terminal order, higher-order fallback, and proof output must remain exact.
- Exact work must improve materially at the guard or its first-order search
  owner, and repeated alternating native timing must confirm the direction.

## Results

Rustfmt, all 44 focused PDTree tests, strict all-target/all-feature pedantic
Clippy, and both release builds pass. Parent and candidate produce
byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

Matched Callgrind instructions fall from `7,662,155,102` to
`7,606,116,113`, a reduction of `56,038,989` (`0.731374%`). Relative to the
matched C count of `5,254,418,333`, the candidate ratio is `1.447566x`. The
release executable shrinks by 824 bytes (`0.00996%`), from 8,271,160 to
8,270,336 bytes.

The first-order search body remains at exactly `1,514,251,396` self
instructions, while its inclusive cost falls from `1,604,529,380` to
`1,558,739,775`, a reduction of `45,789,605` (`2.853772%`). The intended
return-boundary guard falls from `115,967,853` to `63,527,127` inclusive
instructions, a reduction of `52,440,726` (`45.220054%`). Guard self cost
falls from `95,714,145` to `63,526,314`, a reduction of `32,187,831`
(`33.629127%`), and the prior `20,252,029`-instruction vector drop-glue owner
disappears. The guard call count remains exactly `880,523`.

Two independent native blocks provide 128 alternating LUSK6 pairs. Every run
has the exact proof hash. The candidate wins 73 pairs; across all pairs:

- wall mean, median, paired mean, and paired median improve by `0.217767%`,
  `0.266027%`, `0.208137%`, and `0.269368%`;
- CPU mean, median, paired mean, and paired median improve by `0.217359%`,
  `0.265246%`, `0.207711%`, and `0.273548%`.

Restricting both blocks to their final halves yields 64 pairs and 37 wins:

- wall mean, median, paired mean, and paired median improve by `0.170001%`,
  `0.203867%`, `0.160272%`, and `0.285474%`;
- CPU mean, median, paired mean, and paired median improve by `0.168328%`,
  `0.212589%`, `0.158569%`, and `0.289483%`.

Both complete blocks agree on the direction. Block one records 38/64 wins and
paired mean wall/CPU improvements of `0.224498%`/`0.225237%`; block two
records 35/64 wins and improvements of `0.191776%`/`0.190184%`. The final 32
pairs of block two are effectively flat: paired means regress
`0.028404%`/`0.031318%`, while paired medians improve
`0.142675%`/`0.146441%`. The complete independent blocks, combined stable
tail, and exact owner profile are the acceptance evidence rather than that
one noisy tail.

Raw focused evidence is under:

```text
.artifacts/experiments/2026-07-25-019-monotonic-pdt-query-owners/experiment-320/
```

The retained focused archive is
`.artifacts/experiments/2026-07-25-019-monotonic-pdt-query-owners/remote.tar.gz`
with SHA-256
`6EE32C4F7E1B8FFF40059210064957794A58A39784CD287949127E1EC2A42ED0`.

Fresh comprehensive run `.artifacts/linode/260726-100748-dc6e/` validates the
exact focused candidate executable SHA-256
`5a066925ef03ee0b61c82f40cc1bc2b46cd202c1029820e2c56cfd57c48d93d7`:

- 4,416 Rust tests across 33 result groups, Rustfmt, strict
  all-target/all-feature pedantic Clippy, and the native release build pass;
- every binary and test target cross-compiles for Windows GNU x64;
- clean same-tree FOL and higher-order pinned-C references build and pass
  smoke checks;
- all 50 main cases have zero unexpected differences and one declared
  difference;
- all 216 support-tool cases have zero unexpected differences and 15 declared
  differences;
- all ten benchmark cases preserve behavior; and
- smoke Callgrind records `9,904,081` Rust versus `7,590,630` C instructions.

The fresh ten-case aggregate is `1.1148901005x` Rust/C wall time, down from
experiment 319's cross-worker `1.1411223971x`. That fresh-worker movement is
consistent with the candidate but is not used as causal evidence; the
same-worker deterministic and alternating results establish direction.
`VALIDATION_COMPLETE` and `SUCCESS` both contain `ok`.

## Falsification checks and limits

- Every raw cursor is initially owned by the exact search root, a live parent
  cursor, or an earlier monotonic parked owner. A newly discovered child
  remains owned by its live parent until the safe-return guard acquires it.
- The root comparison is pointer-exact and prevents an unnecessary extra
  owner. Search reset clears raw stacks and steps before parked descendants,
  then releases the root.
- A detached old descendant remains parked if the caller mutates an unshared
  root between calls. The regression proves the next exact match succeeds
  after the external old-child owner is dropped and proves a later return
  does not acquire a duplicate owner.
- Shared or cyclic cursor identity is deduplicated. A root self-cycle remains
  covered by the exact search-state owner.
- Scoped argument/type borrows retain safe `RefCell` conflict behavior.
  Higher-order specialization never dispatches to the borrowed cursor.
- Monotonic retention may keep detached descendants alive longer than the
  current stack strictly needs, but only until the existing active-search
  exit boundary. Pointer deduplication is linear in the number of discovered
  descendants; the matched workload shows its complete cost in the guard
  profile and improves exact work by 45.22% there.
- The same parent executable has a 172,933-instruction cross-worker difference
  from experiment 319 despite an identical binary hash. Only the matched
  same-worker parent/candidate comparison is treated as causal evidence.
- The native gain is modest and timing noise can reverse a 32-pair tail.
  Deterministic whole-program and owner-local work, two positive complete
  native blocks, and the positive combined stable tail establish direction;
  the result is not presented as a large end-to-end speedup.

## Decision

Accept. The change removes complete owner-set reconstruction at each safe
return while strengthening the lifetime invariant: a discovered descendant
cannot be released until the raw search structures are cleared. Exact
whole-program work falls `0.731%`, guard work falls `45.22%`, both independent
native blocks improve, exact proof output is preserved, and the complete
compatibility, resource, portability, and quality matrices remain green. The
normal `1.10x` whole-prover performance target remains open.
