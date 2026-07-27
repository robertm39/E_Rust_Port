# Experiment 323: Current proof-search line attribution

## Status

Complete diagnostic experiment for Bead `E_Rust_Port-j76.5.5`; production
source is unchanged.

## Question

After the accepted optimization series through Experiment 320, which current
source lines account for the remaining Rust/C proof-search differential and
offer a source-shaped optimization that has not already been rejected?

## Baseline

- Accepted production: Experiment 320 commit `2e2c5270`.
- Latest comprehensive fresh-worker aggregate: `1.114890x` C.
- Matched accepted LUSK6 work: approximately `7.606` billion Rust
  instructions versus `5.254` billion for C.
- Experiment 310 established that the matched PDTree subtree was already
  cheaper in Rust, while term insertion/replacement and substitution
  normalization were the largest Rust/C differentials.

## Method

A dedicated Ubuntu 24.04 worker
`e-rust-codex-260726-115849-8c48` built the accepted default-feature release
twice with Rust 1.97.1: once normally and once with
`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`. Both binaries then ran the
same exact LUSK6 Callgrind command:

```bash
valgrind --tool=callgrind \
  BINARY eprover/EXAMPLE_PROBLEMS/SMOKETEST/LUSK6.lop \
  --auto --silent --cpu-limit=600 --memory-limit=2048 \
  --detsort-rw --detsort-new
```

The controller lifecycle was:

```powershell
.\linode-runner.ps1 up
try {
    .\linode-runner.ps1 sync
    .\linode-runner.ps1 exec -- bash `
      /opt/e-rust-port/source/experiments/2026-07-25-022-current-line-attribution/remote_profile.sh `
      /opt/e-rust-port/source `
      /opt/e-rust-port/artifacts/experiment-323
}
finally {
    .\linode-runner.ps1 down
}
```

The uploaded snapshot SHA-256 was
`bc9473f1715cc14202f0b4a684a9c2ef334caa3e4db9741df5bdf7d9fbb7162b`.
The retained `analyze_lines.py` expands Callgrind's compressed file,
function, and line identifiers and excludes the cost record following each
`calls=` record, so its rankings report self work rather than inclusive
callee work:

```powershell
python experiments/2026-07-25-022-current-line-attribution/analyze_lines.py `
  .artifacts/linode/260726-115849-8c48/callgrind-lines.out `
  --source-root . --limit 100
```

## Falsification criteria

- The line-table and normal release binaries must produce byte-identical proof
  output with empty program stderr.
- Line tables must add negligible exact work relative to the normal release
  build on the same worker.
- The next proposed optimization must target a current high-cost source line
  and a Rust/C differential, not merely a function that is equally expensive
  in both implementations.
- Previously rejected ownership, layout, and forced-inlining candidates must
  not be repeated unchanged.

## Results

The normal and line-table binaries are distinct and differ substantially in
size, but line metadata changes exact work by only 4,501 instructions:

| Build | SHA-256 prefix | Bytes | Callgrind instructions |
| --- | --- | ---: | ---: |
| Normal | `5a066925ef03` | 8,270,336 | 7,605,982,425 |
| Line tables | `ce492ff17bb7` | 33,260,520 | 7,605,986,926 |

The line-table overhead is `0.00005918%`. Both binaries exit zero, leave
program stderr empty, and produce the same 378-byte proof with SHA-256
`b50bfb29a2fa3728792a422be2bd83a54c436fa1536c79b8d95c1c1f8c7f427d`.
Relative to Experiment 310's matched C count of `5,254,418,333`, the current
accepted Rust executable is `1.447540x` by exact instruction count.

The line profile reinforces the matched-boundary warning. PDTree remains hot,
but Experiment 310 measured its complete Rust cursor subtree about 300 million
instructions below C's corresponding subtree. Experiments 321 and 322 also
rejected the two newest source-shaped PDTree ownership/substitution variants.
That heat is not evidence of the remaining differential.

The current actionable ranking instead exposes repeated term-argument
representation work:

- `BorrowedTermCell::compare_top_order` is called 7,113,427 times and accounts
  for 189,198,884 self and 349,235,156 inclusive instructions.
- Its two `TermArgs::as_slice` shape dispatches account for 60,313,235 and
  25,383,830 line-attributed self instructions before the generic length,
  zip, option, and identity-comparison loop. Together those two lines alone
  are `1.1267%` of the whole proof.
- The splay/top-insertion owner remains the largest matched Rust/C
  representation differential. C reads its arity and raw argument array
  directly; Rust's accepted cursor removed dynamic borrows in Experiment 318
  but still independently converts both small-arity enums to slices on every
  comparison.
- Substitution normalization remains the second differential: the profile
  attributes 67,986,964 self instructions in `termtypes.rs` and 25,588,114 in
  `subst.rs` to `Substitution::norm_term`, in addition to inlined library and
  fresh-variable work.
- `term_follow_top_rw_chain` has 67,512,702 source self instructions, but
  Experiment 244 already removed the apparent owned-handle clone in the
  recursive caller. Its tiny deterministic gain reversed decisively in native
  measurement, so that candidate must not be repeated unchanged.

Line attribution under fat LTO is not a complete component accounting:
inlined standard-library work can appear under its own source file, and
function prologues/epilogues can map to the declaration, closing brace, or
line zero. The proposed comparator candidate relies on the matched C/Rust
insertion differential plus call counts and multiple adjacent lines, not on a
single ambiguous closing-brace sample.

## Decision

Accept this as diagnostic evidence and leave production source unchanged. The
next experiment should preserve `TermArgs` storage and exact term-tree key
order, but compare the two argument representations as one pair: directly
handle `Empty`, unroll `One` and `Two`, and retain a length-checked heap
fallback. That avoids two independent enum-to-slice dispatches and the generic
zip loop on the dominant small-arity path without repeating a rejected
ownership, link-layout, or forced-inlining candidate.

Raw evidence:

```text
.artifacts/experiments/2026-07-25-022-current-line-attribution/experiment-323/
.artifacts/experiments/2026-07-25-022-current-line-attribution/remote.tar.gz
```

The archive SHA-256 is
`56DB036AC33DB1A4DAB49F785444F23C23231D628BE66A32E3287532A8125EC3`.
The worker and temporary firewall were deleted after collection.
