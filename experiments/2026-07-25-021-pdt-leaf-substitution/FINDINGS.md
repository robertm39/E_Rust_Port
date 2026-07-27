# Experiment 322: Retain PDTree leaf substitution

## Status

Rejected for Bead `E_Rust_Port-j76.5.5`; production remains at accepted
Experiment 320 commit `2e2c5270`.

## Question

Can the borrowed first-order PDTree cursor retain its live substitution while
returning multiple stored entries from the same indexed leaf, matching C
`PDTreeFindNextDemodulator`, and backtrack only when advancing to another leaf?

## Baseline

- Accepted parent: commit `2e2c5270`.
- Matched LUSK6 work is `7,606,116,113` Rust instructions versus
  `5,254,418,333` for C (`1.447566x`).
- The borrowed first-order search specialization retires `1,514,251,396`
  self instructions and `1,558,739,775` inclusive instructions.
- Reconstructing returned-leaf bindings reaches
  `Substitution::add_owned_binding_to` `284,354` times from this search owner
  and retires `17,680,516` instructions there.

## Candidate

The candidate recognizes a resumed terminal-entry traversal from its existing
cursor state. It leaves the indexed substitution and any caller-added
right-side bindings active for later entries at that same leaf. Once terminal
entries are exhausted, it backtracks to the search base before changing the
traversal path. If a caller externally removed the indexed bindings, the next
entry reconstructs them.

A focused regression stores two occurrences at one generalized leaf, adds a
caller binding after the first occurrence, verifies both bindings survive the
second occurrence, and verifies that advancing to a specific leaf clears them.

## Method

The retained reproduction worker was
`e-rust-codex-260726-114022-2431`. The accepted-parent archive had SHA-256
`3E4B913E0A1EF7D17699FEA9D86CA543AB4A47596F42094DB94D40B5FADC4637`.
The uploaded worktree snapshot was
`86e9ce0af3707c2483caeb0b9c95e104f721986ae3997d2ea38d65cbde57e61d`.
The candidate source SHA-256 was
`817080fc71dd2cf72a8c3563b2f0b56bf15681fae7a987f27d136a85e90bb34d`.

The exact controller sequence was:

```powershell
git archive --format=tar --output=accepted-source.tar 2e2c5270 `
  src/clauses/pdtrees.rs
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-021-pdt-leaf-substitution/remote_measure.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-322
}
finally {
    # The experiment artifact root was collected before this teardown.
    .\linode-runner.ps1 down
}
```

The retained script runs Rustfmt, all 45 focused PDTree tests, strict
all-target/all-feature pedantic Clippy, parent/candidate release builds,
Callgrind with exact proof comparison, and 64 alternating native pairs.

## Falsification criteria

- Every entry at one leaf must observe the same live indexed and caller-added
  bindings, exactly as the C terminal-entry traversal does.
- Advancing away from the leaf must clear both indexed and caller-added
  bindings back to the substitution position captured at search start.
- Existing query mutation, search reset, traversal order, substitution
  backtracking, higher-order fallback, exact proof output, and full gates must
  remain unchanged.
- Matched Callgrind work must improve materially and repeated alternating
  native timing must confirm the direction.

## Results

Rustfmt, all 45 focused PDTree tests, strict all-target/all-feature pedantic
Clippy, and both release builds pass. Parent and candidate produce
byte-identical LUSK6 proof output with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`,
empty stderr, and exit zero.

The candidate is decisively worse. Matched Callgrind instructions rise from
`7,605,982,415` to `7,642,063,916`, an increase of `36,081,501`
(`0.474383%`). Relative to the accepted C reference work of `5,254,418,333`
instructions, the candidate is `1.454407x` C versus `1.447540x` for the
same-worker parent.

The first-order search specialization itself rises from `1,514,251,396` to
`1,550,434,174` self instructions, an increase of `36,182,778`
(`2.389483%`). The intended saving is tiny: calls from this owner into
`Substitution::add_owned_binding_to` fall by only `3,168`, from `284,354` to
`281,186` (`1.114104%`), and their instructions fall by only `142,560`
(`0.806311%`). Same-leaf duplicate candidates are too rare to repay the
per-call state branch and enlarged hot loop.

The candidate executable grows by 368 bytes, from 8,270,336 to 8,270,704
bytes. Across 64 alternating native pairs it wins only 27 wall and 27 CPU
pairs:

- paired mean wall and CPU time regress by `0.596193%` and `0.594703%`;
- wall and CPU medians regress by `0.509553%` and `0.515491%`.

The final 32 pairs remain negative, with 14 wall and 14 CPU wins:

- paired mean wall and CPU time regress by `0.511576%` and `0.511248%`;
- wall and CPU medians regress by `0.524028%` and `0.522584%`.

An earlier decision run on worker `e-rust-codex-260726-112016-f1ca` recorded
the same deterministic regression to within ten instructions
(`+36,081,491`) and a larger `1.065%` paired mean wall/CPU regression. Its
worker was mistakenly deleted before raw artifact collection, so only the
fully retained reproduction is used as the durable evidence set.

Raw evidence is under:

```text
.artifacts/experiments/2026-07-25-021-pdt-leaf-substitution/experiment-322/
```

The retained archive is
`.artifacts/experiments/2026-07-25-021-pdt-leaf-substitution/remote.tar.gz`
with SHA-256
`4DB266CA425F40742698BF738BA5046899263BD4A2B211BF9350E560C8DD4D82`.

## Falsification checks and limits

- Exact proof output and all focused functional contracts pass.
- The regression directly covers C's same-leaf lifetime for indexed and
  caller-added bindings, plus backtracking when traversal reaches another
  leaf.
- The candidate saves only 1.11% of indexed binding installations, while its
  added state path executes throughout the 783,453-call search owner.
- The retained parent records ten fewer instructions than the initial worker;
  only same-worker parent/candidate differences are treated as causal.
- No comprehensive validation is warranted after both deterministic and
  native criteria fail. All remote resources were deleted after retained raw
  artifact collection.

## Decision

Reject the candidate. It matches C's terminal-entry substitution lifetime but
adds substantially more work than it removes because LUSK6 has only 3,168
reusable same-leaf binding installations. Production is restored byte-for-byte
to accepted Experiment 320 commit `2e2c5270`; only the experiment record and
reusable measurement scripts are retained. The normal `1.10x` performance
target remains open.
