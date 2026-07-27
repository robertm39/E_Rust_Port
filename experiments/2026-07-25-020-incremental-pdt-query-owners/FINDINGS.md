# Experiment 321: Incremental PDTree query owners

## Status

Rejected for Bead `E_Rust_Port-j76.5.5`; production remains at accepted
Experiment 320 commit `2e2c5270`.

## Question

Can the borrowed first-order PDTree cursor record only descendants discovered
since the previous safe call, while retaining the complete active-cursor scan
solely as an unwind fallback?

## Baseline

- Accepted parent: commit `2e2c5270`.
- Matched LUSK6 work is `7,606,116,113` Rust instructions versus
  `5,254,418,333` for C (`1.447566x`).
- The borrowed first-order search specialization retires `1,514,251,396`
  self instructions and `1,558,739,775` inclusive instructions.
- Its normal return-boundary guard retires `63,527,127` instructions across
  `880,523` calls because it scans the current pending and processed cursor
  stacks even when no new descendant was discovered.

## Candidate

Two source-shaped variants were measured.

The initial discovery-list variant records each child cursor as it is pushed
during a successful symbol expansion. Normal completion parks only that list
and disarms the guard. If a cursor operation unwinds before all newly pushed
children can be recorded, the still-armed guard scans the complete active raw
stacks before control reaches a safe caller.

The refined dirty-bit variant retains the accepted active-stack scan but runs
it only after a symbol expansion. The bit is set before borrowing/pushing
arguments, so the RAII guard still scans a partially expanded stack during
unwinding. Variable consumption, backtracking, and terminal enumeration add no
new cursor allocations and do not set the bit.

Both variants add a focused regression that leaves one argument uninitialized,
forces a partial first-order expansion, catches its panic, and verifies that
the already-pushed child was parked by the unwind path.

## Method

The focused worker was
`e-rust-codex-260726-103956-b6ca`. The accepted-parent archive had SHA-256
`3E4B913E0A1EF7D17699FEA9D86CA543AB4A47596F42094DB94D40B5FADC4637`.
The final dirty-bit snapshot was
`549cc609ecb9de72d66e02111f64e98672f6d84b0713c371cd53aca4b7727dde`.
Each variant used the same controller lifecycle and a distinct remote artifact
root:

```powershell
git archive --format=tar --output=accepted-source.tar 2e2c5270 `
  src/clauses/pdtrees.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-020-incremental-pdt-query-owners/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-321-dirty-flag
}
finally {
    .\linode-runner.ps1 down
}
```

The retained script runs Rustfmt, all 45 focused PDTree tests, strict
all-target/all-feature pedantic Clippy, parent/candidate release builds,
Callgrind with exact proof comparison, and 64 alternating native pairs. The
initial discovery-list run was stopped before native timing after its exact
profile decisively falsified the design.

## Falsification criteria

- Normal returns must empty the discovery list and retain each pointer-exact
  descendant at most once.
- A partial expansion that panics after pushing a child must park that child
  through the full-stack unwind path before the panic can be caught.
- Query mutation between calls, root-owner elision, search reset, traversal,
  substitutions, backtracking, type/weight constraints, higher-order fallback,
  and exact proof output must remain unchanged.
- Exact work must improve materially at the guard or its first-order search
  owner, and repeated alternating native timing must confirm the direction.

## Results

Both variants pass Rustfmt, all 45 focused PDTree tests including the new
partial-unwind regression, strict all-target/all-feature pedantic Clippy, both
release builds, and exact proof comparison. Parent and candidates produce
byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

The discovery-list variant is decisively worse. Matched Callgrind instructions
rise from `7,605,982,425` to `7,958,023,580`, an increase of `352,041,155`
(`4.628477%`). Its normal `finish` path alone costs `257,552,499`
instructions, while the first-order search body rises from `1,514,251,396` to
`1,645,186,571` self instructions. Repeated rediscovery is substantially more
expensive than scanning only the live cursor path. The candidate source
SHA-256 is
`d4797c481764571117806d31ce2427e9ef0674f4e4973aef9a46466ce7508064`;
the executable grows by 1,512 bytes, from 8,270,336 to 8,271,848 bytes.

The dirty-bit refinement improves the narrow guard but regresses the complete
program. Guard instructions fall from `63,527,127` to `60,638,582`, a
reduction of `2,888,545` (`4.546947%`), but the first-order search body rises
from `1,514,251,396` to `1,516,783,310` self instructions. Total instructions
rise by `654,410` (`0.008604%`), from `7,605,982,425` to `7,606,636,835`.
The candidate/C ratio is `1.447665x`. Its source SHA-256 is
`aff5d45f3fc638fa59668ce2dc81eaf9e871f9161e0198fc24b310e59be3a58e`;
the executable grows by 80 bytes to 8,270,416 bytes.

The dirty-bit native block agrees with rejection. Across 64 alternating pairs,
the candidate wins 26 wall and 24 CPU pairs:

- paired mean wall and CPU time regress by `0.045215%` and `0.046655%`;
- wall and CPU medians regress by `0.485987%` and `0.480993%`.

The final 32 pairs strengthen the reversal, with only 10 wall and 9 CPU wins:

- paired mean wall and CPU time regress by `0.479780%` and `0.478461%`;
- paired median wall and CPU time regress by `0.735233%` and `0.743332%`.

Every native run retains the exact proof hash.

Raw evidence is under:

```text
.artifacts/experiments/2026-07-25-020-incremental-pdt-query-owners/experiment-321/
.artifacts/experiments/2026-07-25-020-incremental-pdt-query-owners/experiment-321-dirty-flag/
```

The retained archive is
`.artifacts/experiments/2026-07-25-020-incremental-pdt-query-owners/remote.tar.gz`
with SHA-256
`14C38D9534BDA471533EFCC02498DEE78DA9A97D466D35DCF6546DB31EB92221`.

## Falsification checks and limits

- Both variants preserve exact proof output and pass the normal-return,
  query-mutation, search-reset, higher-order fallback, and partial-unwind
  focused contracts.
- The discovery list retains at most one owner per pointer, but recording
  repeated traversal discoveries before deduplication moves far more work into
  every safe return than the active-stack scan.
- The dirty bit safely covers partial pushes because it is set before argument
  borrowing. It does remove 4.55% of guard work, but adds more work to the
  hotter search body and regresses the native stable tail.
- The same accepted parent binary records `133,688` fewer instructions on this
  worker than in Experiment 320. Only same-worker parent/candidate differences
  are treated as causal evidence.
- No comprehensive run is warranted after both deterministic and native
  acceptance criteria fail. All remote resources were deleted after raw
  artifact collection.

## Decision

Reject both variants. The discovery list is substantially more expensive than
the live cursor scan, and the refined dirty bit does not recover the result:
whole-program exact work is slightly worse and the native stable tail regresses
about 0.48%. Production is restored byte-for-byte to commit `2e2c5270`; only
the experiment record and reusable measurement scripts are retained. The
normal `1.10x` performance target remains open.
